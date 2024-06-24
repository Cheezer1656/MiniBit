#![allow(clippy::type_complexity)]

use bevy_ecs::query::WorldQuery;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use valence::entity::living::Health;
use valence::entity::{EntityId, EntityStatuses};
use valence::math::Vec3Swizzles;
use valence::message::ChatMessageEvent;
use valence::protocol::sound::SoundCategory;
use valence::protocol::Sound;
use valence::{prelude::*, CompressionThreshold, ServerSettings};
use valence_anvil::AnvilLevel;

#[derive(Resource)]
struct ServerConfig {
    world_path: PathBuf,
    spawn_pos: DVec3,
}

#[derive(Resource)]
struct ServerGlobals {
    layer_id: Entity,
    queue: Vec<Entity>,
}

#[derive(Bundle)]
struct Game {
    layer: EntityLayerId,
}

#[derive(Component)]
struct PlayerGameState {
    game_id: Option<Entity>,
    game_start: SystemTime,
    game_stage: u8,
    nth_player: usize,
    wins: u32,
}

#[derive(Component, Default)]
struct CombatState {
    last_attacked_tick: i64,
    has_bonus_knockback: bool,
}

#[derive(Event)]
struct StartGameEvent(Entity);

#[derive(Event)]
struct EndGameEvent {
    entity: Entity,
    loser: Entity,
}

pub fn main() {
    let Ok(config) = std::fs::read_to_string("config.json") else {
        eprintln!("Failed to read `config.json`. Exiting.");
        return;
    };
    let Ok(config) = json::parse(&config) else {
        eprintln!("Failed to parse `config.json`. Exiting.");
        return;
    };

    if config["server"].is_null() || config["world"].is_null() {
        eprintln!("`server` or `world` key is missing in `config.json`. Exiting.");
        return;
    }

    let world_path: PathBuf = match config["world"]["path"].as_str() {
        Some(dir) => Path::new(dir).to_path_buf(),
        None => {
            eprintln!("`path` key is missing in `world` object in `config.json`. Exiting.");
            return;
        }
    };

    if !world_path.exists() {
        eprintln!(
            "Directory `{}` does not exist. Exiting.",
            world_path.display()
        );
        return;
    }

    if !world_path.is_dir() {
        eprintln!("`{}` is not a directory. Exiting.", world_path.display());
        return;
    }

    let spawn_pos = config["world"]["spawn"]
        .members()
        .map(|v| v.as_f64().unwrap_or(0.0))
        .collect::<Vec<f64>>();

    let server_config = ServerConfig {
        world_path,
        spawn_pos: DVec3::new(spawn_pos[0], spawn_pos[1], spawn_pos[2]),
    };

    App::new()
        .insert_resource(NetworkSettings {
            address: SocketAddr::new(
                config["server"]["ip"]
                    .as_str()
                    .unwrap_or("0.0.0.0")
                    .parse()
                    .unwrap_or(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))),
                config["server"]["port"].as_u16().unwrap_or(25565),
            ),
            max_players: config["server"]["max_players"].as_usize().unwrap_or(20),
            max_connections: config["server"]["max_players"].as_usize().unwrap_or(20),
            connection_mode: match config["server"]["connection_mode"].as_u8().unwrap_or(0) {
                1 => ConnectionMode::Offline,
                2 => ConnectionMode::BungeeCord,
                3 => ConnectionMode::Velocity {
                    secret: Arc::from(config["server"]["secret"].as_str().unwrap_or("")),
                },
                _ => ConnectionMode::Online {
                    prevent_proxy_connections: config["server"]["prevent_proxy_connections"]
                        .as_bool()
                        .unwrap_or(true),
                },
            },
            ..Default::default()
        })
        .insert_resource(ServerSettings {
            compression_threshold: CompressionThreshold(-1),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .insert_resource(server_config)
        .add_event::<StartGameEvent>()
        .add_event::<EndGameEvent>()
        .add_systems(Startup, setup)
        .add_systems(EventLoopUpdate, handle_combat_events)
        .add_systems(
            Update,
            (
                init_clients,
                despawn_disconnected_clients,
                handle_oob_clients,
                start_game.after(init_clients),
                end_game.after(handle_oob_clients),
                gameloop,
                chat_message,
            ),
        )
        .add_systems(PostUpdate, (handle_disconnect, check_queue))
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    config: Res<ServerConfig>,
) {
    let layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);
    let mut level = AnvilLevel::new(&config.world_path, &biomes);

    for z in -1..1 {
        for x in -1..1 {
            let pos = ChunkPos::new(x, z);

            level.ignored_chunks.insert(pos);
            level.force_chunk_load(pos);
        }
    }

    let layer_id = commands.spawn((layer, level)).id();

    commands.insert_resource(ServerGlobals {
        layer_id,
        queue: Vec::new(),
    });
}

fn init_clients(
    mut clients: Query<
        (
            Entity,
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut GameMode,
            &mut Health,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
    mut commands: Commands,
    mut globals: ResMut<ServerGlobals>,
    config: Res<ServerConfig>,
) {
    for (
        entity,
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut game_mode,
        mut health,
    ) in clients.iter_mut()
    {
        let Ok(layer) = layers.get(globals.layer_id) else {
            continue;
        };

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set(config.spawn_pos);
        *game_mode = GameMode::Adventure;
        health.0 = 20.0;
        commands.entity(entity).insert((
            PlayerGameState {
                game_id: None,
                game_start: SystemTime::now(),
                game_stage: 0,
                nth_player: 0,
                wins: 0,
            },
            CombatState::default(),
        ));

        globals.queue.push(entity);
    }
}

fn handle_disconnect(
    disconncted: Query<(Entity, &PlayerGameState), Added<Despawned>>,
    mut clients: Query<(Entity, &mut Client, &PlayerGameState)>,
    mut end_game: EventWriter<EndGameEvent>,
    mut globals: ResMut<ServerGlobals>,
) {
    for (entity, dc_gamestate) in disconncted.iter() {
        if globals.queue.contains(&entity) {
            globals.queue.retain(|&x| x != entity);
        } else {
            for (player, mut client, gamestate) in clients.iter_mut() {
                if gamestate.game_id == dc_gamestate.game_id {
                    client.send_chat_message("Your opponent disconnected!");
                    client.clear_title();
                    end_game.send(EndGameEvent {
                        entity: player,
                        loser: entity,
                    });
                }
            }
        }
    }
}

fn check_queue(
    mut start_game: EventWriter<StartGameEvent>,
    mut clients: Query<&mut PlayerGameState>,
    server: Res<Server>,
    mut commands: Commands,
    mut globals: ResMut<ServerGlobals>,
) {
    if globals.queue.len() < 2 {
        return;
    }
    globals.queue.shuffle(&mut thread_rng());
    while globals.queue.len() > 1 {
        let entitylayer = commands.spawn(EntityLayer::new(&server)).id();

        let game_id = commands
            .spawn(Game {
                layer: EntityLayerId(entitylayer),
            })
            .id();
        for (i, entity) in globals.queue.drain(0..2).enumerate() {
            let Ok(mut gamestate) = clients.get_mut(entity) else {
                continue;
            };
            gamestate.game_id = Some(game_id);
            gamestate.game_start = SystemTime::now();
            gamestate.game_stage = 0;
            gamestate.nth_player = i;
            start_game.send(StartGameEvent(entity));
        }
    }
}

#[derive(WorldQuery)]
#[world_query(mutable)]
struct CombatQuery {
    client: &'static mut Client,
    id: &'static EntityId,
    pos: &'static Position,
    state: &'static mut CombatState,
    statuses: &'static mut EntityStatuses,
    gamestate: &'static PlayerGameState,
}

fn handle_combat_events(
    server: Res<Server>,
    mut clients: Query<CombatQuery>,
    mut sprinting: EventReader<SprintEvent>,
    mut interact_entity: EventReader<InteractEntityEvent>,
) {
    for &SprintEvent { client, state } in sprinting.read() {
        if let Ok(mut client) = clients.get_mut(client) {
            client.state.has_bonus_knockback = state == SprintState::Start;
        }
    }

    for &InteractEntityEvent {
        client: attacker_client,
        entity: victim_client,
        ..
    } in interact_entity.read()
    {
        let Ok([mut attacker, mut victim]) = clients.get_many_mut([attacker_client, victim_client])
        else {
            continue;
        };

        if attacker.gamestate.game_id != victim.gamestate.game_id || server.current_tick() - victim.state.last_attacked_tick < 10 {
            continue;
        }

        victim.state.last_attacked_tick = server.current_tick();

        let victim_pos = victim.pos.0.xz();
        let attacker_pos = attacker.pos.0.xz();

        let dir = (victim_pos - attacker_pos).normalize().as_vec2();

        let knockback_xz = if attacker.state.has_bonus_knockback {
            18.0
        } else {
            8.0
        };
        let knockback_y = if attacker.state.has_bonus_knockback {
            8.432
        } else {
            6.432
        };

        victim
            .client
            .set_velocity([dir.x * knockback_xz, knockback_y, dir.y * knockback_xz]);

        attacker.state.has_bonus_knockback = false;

        victim.client.play_sound(
            Sound::EntityPlayerHurt,
            SoundCategory::Player,
            victim.pos.0,
            1.0,
            1.0,
        );
        attacker.client.play_sound(
            Sound::EntityPlayerHurt,
            SoundCategory::Player,
            victim.pos.0,
            1.0,
            1.0,
        );
    }
}

fn handle_oob_clients(
    usernames: Query<&Username, With<Username>>,
    mut clients: Query<(Entity, &mut Client, &PlayerGameState), With<Client>>,
    mut positions: Query<(Entity, &mut Position, &PlayerGameState), With<Client>>,
    mut end_game: EventWriter<EndGameEvent>,
    config: Res<ServerConfig>,
) {
    let mut losers = HashMap::new();
    for (entity, mut pos, gamestate) in positions.iter_mut() {
        if pos.0.y < 0.0 {
            pos.set(config.spawn_pos);
            if gamestate.game_id.is_some() {
                losers.insert(gamestate.game_id, entity);
            }
        }
    }
    for (entity, mut client, gamestate) in clients.iter_mut() {
        if losers.contains_key(&gamestate.game_id) {
            match losers.get(&gamestate.game_id) {
                Some(loser) => {
                    client.send_chat_message(format!(
                        "{} died! Btw you have {} wins!",
                        usernames
                            .get(*loser)
                            .unwrap_or(&Username(String::from("Unknown"))),
                        gamestate.wins + if *loser == entity { 0 } else { 1 }
                    ));
                    end_game.send(EndGameEvent {
                        entity,
                        loser: *loser,
                    });
                }
                _ => {}
            }
        }
    }
}

fn start_game(
    mut clients: Query<(
        &mut Client,
        &PlayerGameState,
        &mut EntityLayerId,
        &mut VisibleChunkLayer,
        &mut VisibleEntityLayers,
        &mut Position,
    )>,
    games: Query<&EntityLayerId, Without<Client>>,
    chunklayers: Query<Entity, With<ChunkLayer>>,
    entitylayers: Query<Entity, With<EntityLayer>>,
    mut start_game: EventReader<StartGameEvent>,
    globals: Res<ServerGlobals>,
    config: Res<ServerConfig>,
) {
    for event in start_game.read() {
        let Ok((
            mut client,
            gamestate,
            mut layer_id,
            mut visible_chunk_layer,
            mut visible_entity_layers,
            mut pos,
        )) = clients.get_mut(event.0)
        else {
            continue;
        };
        let Some(game_id) = gamestate.game_id else {
            continue;
        };
        let Ok(chunk_id) = games.get(game_id) else {
            continue;
        };
        let Ok(chunklayer) = chunklayers.get(globals.layer_id) else {
            continue;
        };
        let Ok(entitylayer) = entitylayers.get(chunk_id.0) else {
            continue;
        };

        layer_id.0 = entitylayer;
        visible_chunk_layer.0 = chunklayer;
        visible_entity_layers.0.clear();
        visible_entity_layers.0.insert(entitylayer);

        let mut newpos = config.spawn_pos;
        if gamestate.nth_player == 0 {
            newpos.z = 8.0;
        } else {
            newpos.z = -8.0;
        }
        pos.set(newpos);

        client.send_chat_message("Game started!");
    }
}

fn end_game(
    mut clients: Query<(
        &mut PlayerGameState,
        &mut EntityLayerId,
        &mut VisibleChunkLayer,
        &mut VisibleEntityLayers,
        &mut Position,
    )>,
    games: Query<&EntityLayerId, Without<PlayerGameState>>,
    mut end_game: EventReader<EndGameEvent>,
    mut commands: Commands,
    mut globals: ResMut<ServerGlobals>,
    config: Res<ServerConfig>,
) {
    let mut to_despawn: HashSet<Entity> = HashSet::new();
    for event in end_game.read() {
        let Ok((
            mut gamestate,
            mut layer_id,
            mut visible_chunk_layer,
            mut visible_entity_layers,
            mut pos,
        )) = clients.get_mut(event.entity)
        else {
            continue;
        };
        let Some(game_id) = gamestate.game_id else {
            continue;
        };
        let Ok(entitylayer_id) = games.get(game_id) else {
            continue;
        };
        layer_id.0 = globals.layer_id;
        visible_chunk_layer.0 = globals.layer_id;
        visible_entity_layers.0.clear();
        visible_entity_layers.0.insert(entitylayer_id.0);
        pos.set(config.spawn_pos);

        if event.loser != event.entity {
            gamestate.wins += 1;
        }

        gamestate.game_id = None;

        globals.queue.push(event.entity);
        to_despawn.insert(entitylayer_id.0);
        to_despawn.insert(game_id);
    }
    for entity in to_despawn {
        commands.entity(entity).despawn();
    }
}

fn gameloop(
    mut clients: Query<(
        &mut Client,
        &mut PlayerGameState,
        &mut Position,
        &mut Look,
        &mut HeadYaw,
    )>,
    config: Res<ServerConfig>,
) {
    for (mut client, mut gamestate, mut pos, mut look, mut yaw) in clients.iter_mut() {
        if gamestate.game_id.is_some() {
            if gamestate.game_stage < 4 {
                let mut newpos = config.spawn_pos;
                if gamestate.nth_player == 0 {
                    newpos.z = 8.0;
                    yaw.0 = 180.0;
                    look.yaw = 180.0;
                    look.pitch = 0.0;
                } else {
                    newpos.z = -8.0;
                    yaw.0 = 0.0;
                    look.yaw = 0.0;
                    look.pitch = 0.0;
                }
                pos.set(newpos);
            }
            if gamestate.game_stage == 0 {
                client.set_title("3".color(Color::GREEN));
                gamestate.game_stage = 1;
            } else if gamestate.game_stage == 1
                && gamestate
                    .game_start
                    .elapsed()
                    .unwrap_or(std::time::Duration::new(0, 0))
                    .as_secs()
                    >= 1
            {
                client.set_title("2".color(Color::GOLD));
                gamestate.game_stage = 2;
            } else if gamestate.game_stage == 2
                && gamestate
                    .game_start
                    .elapsed()
                    .unwrap_or(std::time::Duration::new(0, 0))
                    .as_secs()
                    >= 2
            {
                client.set_title("1".color(Color::RED));
                gamestate.game_stage = 3;
            } else if gamestate.game_stage == 3
                && gamestate
                    .game_start
                    .elapsed()
                    .unwrap_or(std::time::Duration::new(0, 0))
                    .as_secs()
                    >= 3
            {
                client.set_title("GO!".color(Color::RED));
                gamestate.game_stage = 4;
            } else if gamestate.game_stage == 4
                && gamestate
                    .game_start
                    .elapsed()
                    .unwrap_or(std::time::Duration::new(0, 0))
                    .as_secs()
                    >= 4
            {
                client.clear_title();
                gamestate.game_stage = 5;
            }
        }
    }
}

fn chat_message(
    players: Query<(&PlayerGameState, &Username)>,
    mut clients: Query<(&mut Client, &PlayerGameState)>,
    mut events: EventReader<ChatMessageEvent>,
) {
    for event in events.read() {
        let Ok((sender_gamestate, sender_name)) = players.get(event.client) else {
            continue;
        };
        for (mut client, gamestate) in clients.iter_mut() {
            if gamestate.game_id == sender_gamestate.game_id {
                client.send_chat_message(
                    (String::new()
                        + &sender_name.0
                        + &String::from(": ")
                        + &event.message.to_string())
                        .color(Color::GRAY),
                );
            }
        }
    }
}
