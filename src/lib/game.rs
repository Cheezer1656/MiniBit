use crate::{load_config, ServerConfig};
use bevy_ecs::query::WorldQuery;
use std::time::SystemTime;
use valence::{
    entity::living::Health,
    message::ChatMessageEvent,
    prelude::*,
    protocol::{sound::SoundCategory, Sound},
    CompressionThreshold, ServerSettings,
};
use valence_anvil::AnvilLevel;

#[derive(Component)]
pub struct Entities(pub Vec<Entity>);

#[derive(Component)]
pub struct GameTime(pub SystemTime);

#[derive(Component)]
pub struct GameStage(pub u8);

#[derive(Bundle)]
pub struct Game {
    pub layer: EntityLayerId,
    pub clients: Entities,
    pub game_start: GameTime,
    pub game_stage: GameStage,
}

#[derive(Component, Default)]
pub struct PlayerGameState {
    pub game_id: Option<Entity>,
    pub team: u8,
    pub wins: u32,
}

#[derive(Component, Default)]
pub struct CombatState {
    pub last_attacked_tick: i64,
    pub has_bonus_knockback: bool,
}

#[derive(Event)]
pub struct StartGameEvent(pub Entity);

#[derive(Event)]
pub struct EndGameEvent {
    pub game_id: Entity,
    pub loser: u8,
}

#[derive(Event)]
pub struct GameStageEvent {
    pub game_id: Entity,
    pub stage: u8,
}

#[derive(Resource)]
pub struct ServerGlobals {
    pub map_layers: Vec<Entity>,
    pub queue: Vec<Entity>,
}

pub fn register_defaults(app: &mut App) -> Result<&mut App, &'static str> {
    let config = match load_config() {
        Ok(config) => config,
        Err(e) => {
            return Err(e);
        }
    };

    return Ok(app
        .insert_resource(config.0)
        .insert_resource(ServerSettings {
            compression_threshold: CompressionThreshold(-1),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .insert_resource(config.1)
        .add_event::<StartGameEvent>()
        .add_event::<EndGameEvent>()
        .add_event::<GameStageEvent>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                init_clients,
                despawn_disconnected_clients,
                start_game.after(init_clients),
                end_game,
                gameloop.after(start_game),
                gamestage_change.after(gameloop),
                chat_message,
            ),
        )
        .add_systems(PostUpdate, (handle_disconnect, check_queue)));
}

pub fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    config: Res<ServerConfig>,
) {
    let mut layers: Vec<Entity> = Vec::new();
    for world_path in config.world_paths.iter() {
        let layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);
        let mut level = AnvilLevel::new(world_path, &biomes);

        for z in -1..1 {
            for x in -1..1 {
                let pos = ChunkPos::new(x, z);

                level.ignored_chunks.insert(pos);
                level.force_chunk_load(pos);
            }
        }

        layers.push(commands.spawn((layer, level)).id());
    }

    commands.insert_resource(ServerGlobals {
        map_layers: layers,
        queue: Vec::new(),
    });
}

pub fn init_clients(
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
        let Ok(layer) = layers.get(globals.map_layers[0]) else {
            continue;
        };

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set(config.spawn_pos);
        *game_mode = GameMode::Adventure;
        health.0 = 20.0;
        commands
            .entity(entity)
            .insert((PlayerGameState::default(), CombatState::default()));

        globals.queue.push(entity);
    }
}

pub fn check_queue(
    mut start_game: EventWriter<StartGameEvent>,
    server: Res<Server>,
    mut commands: Commands,
    mut globals: ResMut<ServerGlobals>,
) {
    if globals.queue.len() < 2 {
        return;
    }
    fastrand::shuffle(&mut globals.queue);
    while globals.queue.len() > 1 {
        let entitylayer = commands.spawn(EntityLayer::new(&server)).id();

        let game_id = commands
            .spawn(Game {
                layer: EntityLayerId(entitylayer),
                clients: Entities(globals.queue.drain(..2).collect()),
                game_start: GameTime(SystemTime::now()),
                game_stage: GameStage(0),
            })
            .id();

        start_game.send(StartGameEvent(game_id));
    }
}

pub fn handle_disconnect(
    disconncted: Query<(Entity, &PlayerGameState), Added<Despawned>>,
    mut clients: Query<(&mut Client, &PlayerGameState)>,
    mut end_game: EventWriter<EndGameEvent>,
    mut globals: ResMut<ServerGlobals>,
) {
    for (entity, dc_gamestate) in disconncted.iter() {
        if globals.queue.contains(&entity) {
            globals.queue.retain(|&x| x != entity);
        } else {
            for (mut client, gamestate) in clients.iter_mut() {
                if gamestate.game_id == dc_gamestate.game_id {
                    client.send_chat_message("Your opponent disconnected!");
                    client.clear_title();
                }
            }
            end_game.send(EndGameEvent {
                game_id: dc_gamestate.game_id.unwrap(),
                loser: dc_gamestate.team,
            });
        }
    }
}

pub fn start_game(
    mut clients: Query<(
        &mut Client,
        &mut PlayerGameState,
        &mut EntityLayerId,
        &mut VisibleChunkLayer,
        &mut VisibleEntityLayers,
        &mut Position,
    )>,
    games: Query<(&EntityLayerId, &Entities), Without<Client>>,
    chunklayers: Query<Entity, With<ChunkLayer>>,
    entitylayers: Query<Entity, With<EntityLayer>>,
    mut start_game: EventReader<StartGameEvent>,
    globals: Res<ServerGlobals>,
    config: Res<ServerConfig>,
) {
    for event in start_game.read() {
        let Ok((game_layer, entities)) = games.get(event.0) else {
            continue;
        };
        for (i, entity) in entities.0.iter().enumerate() {
            let Ok((
                mut client,
                mut gamestate,
                mut layer_id,
                mut visible_chunk_layer,
                mut visible_entity_layers,
                mut pos,
            )) = clients.get_mut(*entity)
            else {
                continue;
            };
            let Ok(chunklayer) =
                chunklayers.get(globals.map_layers[fastrand::usize(..globals.map_layers.len())])
            else {
                continue;
            };
            let Ok(entitylayer) = entitylayers.get(game_layer.0) else {
                continue;
            };

            layer_id.0 = entitylayer;
            visible_chunk_layer.0 = chunklayer;
            visible_entity_layers.0.clear();
            visible_entity_layers.0.insert(entitylayer);

            gamestate.game_id = Some(event.0);

            let mut newpos = config.spawn_pos;
            if i == 0 {
                gamestate.team = 0;
                newpos.z = 8.0;
            } else {
                gamestate.team = 1;
                newpos.z = -8.0;
            }
            pos.set(newpos);

            client.send_chat_message("Game started!");
        }
    }
}

pub fn end_game(
    mut clients: Query<(
        &mut Client,
        &mut PlayerGameState,
        &mut EntityLayerId,
        &mut VisibleChunkLayer,
        &mut VisibleEntityLayers,
        &mut Position,
        &mut Health,
    )>,
    games: Query<(&EntityLayerId, &Entities), Without<PlayerGameState>>,
    mut end_game: EventReader<EndGameEvent>,
    mut commands: Commands,
    mut globals: ResMut<ServerGlobals>,
    config: Res<ServerConfig>,
) {
    for event in end_game.read() {
        let Ok((game_layer, entities)) = games.get(event.game_id) else {
            continue;
        };
        for entity in entities.0.iter() {
            let Ok((
                mut client,
                mut gamestate,
                mut layer_id,
                mut visible_chunk_layer,
                mut visible_entity_layers,
                mut pos,
                mut health,
            )) = clients.get_mut(*entity)
            else {
                continue;
            };
            layer_id.0 = globals.map_layers[0];
            visible_chunk_layer.0 = globals.map_layers[0];
            visible_entity_layers.0.clear();
            visible_entity_layers.0.insert(globals.map_layers[0]);
            pos.set(config.spawn_pos);
            health.0 = 20.0;

            if gamestate.team == event.loser {
                client.send_chat_message("You lost!");
            } else {
                client.send_chat_message("You won!");
                gamestate.wins += 1;
            }

            gamestate.game_id = None;

            globals.queue.push(*entity);
        }

        commands.entity(game_layer.0).despawn();
        commands.entity(event.game_id).despawn();
    }
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct GameQuery {
    client: &'static mut Client,
    gamestate: &'static PlayerGameState,
    pos: &'static mut Position,
    look: &'static mut Look,
    yaw: &'static mut HeadYaw,
}

pub fn gameloop(
    mut clients: Query<GameQuery>,
    mut games: Query<(Entity, &mut GameStage, &GameTime)>,
    mut gamestage: EventWriter<GameStageEvent>,
    config: Res<ServerConfig>,
) {
    for (game_id, mut stage, time) in games.iter_mut() {
        if stage.0 < 4 {
            for mut player in clients.iter_mut() {
                if player.gamestate.game_id == Some(game_id) {
                    let mut newpos = config.spawn_pos;
                    if player.gamestate.team == 0 {
                        newpos.z = 8.0;
                        player.yaw.0 = 180.0;
                        player.look.yaw = 180.0;
                        player.look.pitch = 0.0;
                    } else {
                        newpos.z = -8.0;
                        player.yaw.0 = 0.0;
                        player.look.yaw = 0.0;
                        player.look.pitch = 0.0;
                    }
                    player.pos.set(newpos);
                }
            }
        }
        if stage.0 == 0 {
            for mut player in clients.iter_mut() {
                if player.gamestate.game_id == Some(game_id) {
                    player.client.set_title("3".color(Color::GREEN));
                }
            }
            stage.0 = 1;
            gamestage.send(GameStageEvent { game_id, stage: 1 });
        } else if stage.0 == 1
            && time
                .0
                .elapsed()
                .unwrap_or(std::time::Duration::new(0, 0))
                .as_secs()
                >= 1
        {
            for mut player in clients.iter_mut() {
                if player.gamestate.game_id == Some(game_id) {
                    player.client.set_title("2".color(Color::GOLD));
                }
            }
            stage.0 = 2;
            gamestage.send(GameStageEvent { game_id, stage: 2 });
        } else if stage.0 == 2
            && time
                .0
                .elapsed()
                .unwrap_or(std::time::Duration::new(0, 0))
                .as_secs()
                >= 2
        {
            for mut player in clients.iter_mut() {
                if player.gamestate.game_id == Some(game_id) {
                    player.client.set_title("1".color(Color::RED));
                }
            }
            stage.0 = 3;
            gamestage.send(GameStageEvent { game_id, stage: 3 });
        } else if stage.0 == 3
            && time
                .0
                .elapsed()
                .unwrap_or(std::time::Duration::new(0, 0))
                .as_secs()
                >= 3
        {
            for mut player in clients.iter_mut() {
                if player.gamestate.game_id == Some(game_id) {
                    player.client.set_title("GO!".color(Color::RED));
                }
            }
            stage.0 = 4;
            gamestage.send(GameStageEvent { game_id, stage: 4 });
        } else if stage.0 == 4
            && time
                .0
                .elapsed()
                .unwrap_or(std::time::Duration::new(0, 0))
                .as_secs()
                >= 4
        {
            for mut player in clients.iter_mut() {
                if player.gamestate.game_id == Some(game_id) {
                    player.client.clear_title();
                }
            }
            stage.0 = 5;
            gamestage.send(GameStageEvent { game_id, stage: 5 });
        }
    }
}

pub fn gamestage_change(
    mut clients: Query<(&mut Client, &Position)>,
    games: Query<&Entities>,
    mut gamestage: EventReader<GameStageEvent>,
) {
    for event in gamestage.read() {
        if let Ok(entities) = games.get(event.game_id) {
            for entity in entities.0.iter() {
                if let Ok((mut client, pos)) = clients.get_mut(*entity) {
                    match event.stage {
                        1 => client.set_title("3".color(Color::GREEN)),
                        2 => client.set_title("2".color(Color::GOLD)),
                        3 => client.set_title("1".color(Color::RED)),
                        4 => client.set_title("GO!".color(Color::RED)),
                        5 => client.clear_title(),
                        _ => {}
                    }
                    if event.stage < 4 {
                        client.play_sound(
                            Sound::BlockNoteBlockPling,
                            SoundCategory::Master,
                            pos.0,
                            1.0,
                            1.0,
                        );
                    } else if event.stage == 4 {
                        client.play_sound(
                            Sound::BlockNoteBlockPling,
                            SoundCategory::Master,
                            pos.0,
                            1.0,
                            5.0,
                        );
                    }
                }
            }
        }
    }
}

pub fn chat_message(
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
                    (String::new() + &sender_name.0 + &String::from(": ") + &event.message)
                        .color(Color::GRAY),
                );
            }
        }
    }
}
