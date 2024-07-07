#![allow(clippy::type_complexity)]

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};

use valence::{
    entity::{living::Health, player::PlayerEntityBundle},
    event_loop::PacketEvent,
    inventory::HeldItem,
    message::{ChatMessageEvent, SendMessage},
    nbt::compound,
    player_list::{DisplayName, Listed, PlayerListEntryBundle},
    prelude::*,
    protocol::{packets::play::PlayerInteractItemC2s, sound::SoundCategory, Sound},
    CompressionThreshold, ServerSettings,
};
use valence_anvil::AnvilLevel;

#[derive(Clone)]
enum ActionType {
    Message,
    Warp,
    None,
}
#[derive(Component, Clone)]
struct NpcAction {
    command: ActionType,
    args: Vec<String>,
}

#[derive(Component, Clone)]
struct NpcName(String);
#[derive(Component, Clone)]
struct SkinValue(String);
#[derive(Component, Clone)]
struct SkinSignature(String);

#[derive(Bundle, Clone)]
struct NpcConfig {
    uuid: UniqueId,
    name: NpcName,
    position: Position,
    yaw: HeadYaw,
    skin: SkinValue,
    signature: SkinSignature,
    command: NpcAction,
}

struct ParkourConfig {
    name: String,
    start: DVec3,
    end: DVec3,
}

#[derive(Resource)]
struct ServerConfig {
    world_path: PathBuf,
    spawn_pos: DVec3,
    npcs: Vec<NpcConfig>,
    parkour: Vec<ParkourConfig>,
}

#[derive(Resource)]
struct ServerGlobals {
    navigator_gui: Option<Entity>,
}

#[derive(Component)]
struct ParkourStatus {
    name: String,
    start: SystemTime,
    end: DVec3,
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
        npcs: config["npcs"]
            .members()
            .map(|npc| NpcConfig {
                uuid: UniqueId::default(),
                name: NpcName(npc["name"].as_str().unwrap_or("Steve").to_string()),
                position: Position::new(DVec3::new(
                    npc["position"][0].as_f64().unwrap_or(0.0),
                    npc["position"][1].as_f64().unwrap_or(0.0),
                    npc["position"][2].as_f64().unwrap_or(0.0),
                )),
                yaw: HeadYaw(npc["yaw"].as_f32().unwrap_or(0.0)),
                skin: SkinValue(npc["skin"].as_str().unwrap_or("").to_string()),
                signature: SkinSignature(npc["signature"].as_str().unwrap_or("").to_string()),
                command: NpcAction {
                    command: match npc["command"][0].as_str().unwrap_or("") {
                        "message" => ActionType::Message,
                        "warp" => ActionType::Warp,
                        _ => ActionType::None,
                    },
                    args: npc["command"]
                        .members()
                        .skip(1)
                        .map(|v| v.as_str().unwrap_or("").to_string())
                        .collect(),
                },
            })
            .collect(),
        parkour: config["parkour"]
            .members()
            .map(|parkour| ParkourConfig {
                name: parkour["name"].as_str().unwrap_or("Parkour").to_string(),
                start: DVec3::new(
                    parkour["start"][0].as_f64().unwrap(),
                    parkour["start"][1].as_f64().unwrap(),
                    parkour["start"][2].as_f64().unwrap(),
                ),
                end: DVec3::new(
                    parkour["end"][0].as_f64().unwrap(),
                    parkour["end"][1].as_f64().unwrap(),
                    parkour["end"][2].as_f64().unwrap(),
                ),
            })
            .collect(),
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
        .insert_resource(ServerGlobals { navigator_gui: None })
        .add_systems(Startup, setup)
        .add_systems(EventLoopUpdate, item_interactions)
        .add_systems(
            Update,
            (
                despawn_disconnected_clients,
                init_clients,
                manage_players,
                entity_interactions,
                chat_message,
                start_parkour,
                manage_parkour,
                apply_custom_skin,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    server: Res<Server>,
    config: Res<ServerConfig>,
    mut globals: ResMut<ServerGlobals>,
) {
    let layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);
    let mut level = AnvilLevel::new(&config.world_path, &biomes);

    for z in -3..3 {
        for x in -3..3 {
            let pos = ChunkPos::new(x, z);

            level.ignored_chunks.insert(pos);
            level.force_chunk_load(pos);
        }
    }

    let layer_id = commands.spawn((layer, level)).id();

    for npc in &config.npcs {
        let npc_entity = commands
            .spawn(PlayerEntityBundle {
                layer: EntityLayerId(layer_id),
                uuid: npc.uuid,
                position: Position::new(npc.position.get()),
                look: Look::new(180.0, 0.0),
                head_yaw: npc.yaw,
                ..PlayerEntityBundle::default()
            })
            .id();

        commands.entity(npc_entity).insert(npc.clone());

        commands.spawn(PlayerListEntryBundle {
            uuid: npc.uuid,
            username: Username(npc.name.0.to_string()),
            display_name: DisplayName(npc.name.0.clone().color(Color::RED).into()),
            listed: Listed(false),
            ..Default::default()
        });
    }

    globals.navigator_gui = Some(
        commands
            .spawn(Inventory::with_title(InventoryKind::Generic9x6, "Server Navigator"))
            .id(),
    );
}

fn init_clients(
    mut clients: Query<
        (
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut GameMode,
            &mut Health,
            &mut Inventory,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, With<ChunkLayer>>,
    config: Res<ServerConfig>,
) {
    for (
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut game_mode,
        mut health,
        mut inv,
    ) in &mut clients
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set(config.spawn_pos);
        *game_mode = GameMode::Adventure;
        health.0 = 20.0;

        inv.set_slot(
            36,
            ItemStack::new(
                ItemKind::Compass,
                1,
                Some(compound! {
                    "display" => compound! {
                        "Name" => "{\"text\":\"Navigator\",\"italic\":false}"
                    },
                }),
            ),
        )
    }
}

fn manage_players(
    mut clients: Query<(&mut Client, &mut Position, &HeadYaw), With<Client>>,
    mut layers: Query<&mut ChunkLayer>,
    config: Res<ServerConfig>,
) {
    let layer = layers.single_mut();
    for (mut client, mut pos, yaw) in clients.iter_mut() {
        if pos.0.y < 0.0 {
            pos.set([config.spawn_pos.x, config.spawn_pos.y, config.spawn_pos.z]);
        }
        let Some(block) = layer.block(BlockPos::new(
            pos.0.x.floor() as i32,
            pos.0.y.ceil() as i32 - 1,
            pos.0.z.floor() as i32,
        )) else {
            continue;
        };
        if block.state == BlockState::SLIME_BLOCK {
            client.play_sound(
                Sound::EntityFireworkRocketLaunch,
                SoundCategory::Master,
                pos.0,
                1.0,
                1.0,
            );
            let yaw = yaw.0.to_radians();
            client.set_velocity(Vec3::new(-yaw.sin() * 65.0, 30.0, yaw.cos() * 65.0));
        }
    }
}

fn entity_interactions(
    mut clients: Query<(&mut Client, &Username), With<Client>>,
    mut actions: Query<&NpcAction>,
    mut events: EventReader<InteractEntityEvent>,
) {
    for event in events.read() {
        match event.interact {
            valence::prelude::EntityInteraction::Attack => {}
            valence::prelude::EntityInteraction::Interact(hand) => {
                if hand != Hand::Main {
                    continue;
                }
            }
            _ => continue,
        }
        let Ok((mut client, uuid)) = clients.get_mut(event.client) else {
            continue;
        };
        let Ok(action) = actions.get_mut(event.entity) else {
            continue;
        };

        match action.command {
            ActionType::Message => {
                for arg in &action.args {
                    client.send_chat_message(arg.clone().into_text().bold());
                }
            }
            ActionType::Warp => {
                let mut payload: Vec<u8> = Vec::new();
                payload.extend_from_slice("1".as_bytes());
                payload.push(0);
                payload.extend_from_slice(uuid.0.to_string().as_bytes());
                payload.push(0);
                payload.extend_from_slice(action.args[0].as_bytes());
                client.send_custom_payload(ident!("minibit:main"), &payload);
            }
            ActionType::None => {}
        }
    }
}

fn item_interactions(
    mut clients: Query<(Entity, &mut Inventory, &HeldItem), With<Client>>,
    mut packets: EventReader<PacketEvent>,
    mut commands: Commands,
    globals: Res<ServerGlobals>,
) {
    for packet in packets.read() {
        if let Some(_pkt) = packet.decode::<PlayerInteractItemC2s>() {
            if let Ok((entity, mut inv, item)) = clients.get_mut(packet.client) {
                match inv.slot(item.slot()).item {
                    ItemKind::Compass => {
                        commands.entity(entity).insert(OpenInventory::new(globals.navigator_gui.unwrap()));
                    },
                    ItemKind::Barrier => {
                        commands.entity(entity).remove::<ParkourStatus>();
                        inv.set_slot(item.slot(), ItemStack::EMPTY);
                    },
                    _ => {}
                }
            }
        }
    }
}

fn chat_message(
    usernames: Query<&Username>,
    mut clients: Query<&mut Client>,
    mut events: EventReader<ChatMessageEvent>,
) {
    for event in events.read() {
        let Ok(username) = usernames.get(event.client) else {
            continue;
        };
        for mut client in clients.iter_mut() {
            client.send_chat_message(
                (String::new() + &username.0 + &String::from(": ") + &event.message)
                    .color(Color::GRAY),
            );
        }
    }
}

fn start_parkour(
    mut query: Query<(Entity, &mut Client, &mut Inventory, &Position), Without<ParkourStatus>>,
    mut commands: Commands,
    config: Res<ServerConfig>,
) {
    for (entity, mut client, mut inv, pos) in query.iter_mut() {
        for parkour in &config.parkour {
            if pos.0.floor() == parkour.start {
                client.send_chat_message(
                    (String::new() + &parkour.name + " started!")
                        .into_text()
                        .bold()
                        .color(Color::GREEN),
                );
                commands.entity(entity).insert(ParkourStatus {
                    name: parkour.name.clone(),
                    start: SystemTime::now(),
                    end: parkour.end,
                });
                inv.set_slot(
                    44,
                    ItemStack::new(
                        ItemKind::Barrier,
                        1,
                        Some(compound! {
                            "display" => compound! {
                                "Name" => "{\"text\":\"Cancel Parkour\",\"italic\":false}"
                            },
                        }),
                    ),
                );
            }
        }
    }
}

fn manage_parkour(
    mut query: Query<(Entity, &mut Client, &ParkourStatus, &Position), With<ParkourStatus>>,
    mut commands: Commands,
) {
    for (entity, mut client, status, pos) in query.iter_mut() {
        let time = &format!(
            "{:.1}",
            &status
                .start
                .elapsed()
                .unwrap_or(Duration::new(0, 0))
                .as_secs_f32()
        );
        client.set_action_bar(String::new() + &status.name + " - " + time + "s");
        if pos.0.floor() == status.end {
            client.send_chat_message(
                (String::new() + &status.name + " completed in " + time + " seconds!")
                    .into_text()
                    .bold()
                    .color(Color::GREEN),
            );
            commands.entity(entity).remove::<ParkourStatus>();
        }
    }
}

fn apply_custom_skin(
    // This function is not working (SkinValue and SkinSignature are not found in the query)
    mut query: Query<
        (&SkinValue, &SkinSignature, &mut Properties),
        (Added<Properties>, Without<Client>),
    >,
) {
    for (skin, sign, mut props) in query.iter_mut() {
        props.set_skin(skin.0.clone(), sign.0.clone());
    }
}
