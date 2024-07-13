#![allow(clippy::type_complexity)]

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use valence::{entity::{player::PlayerEntityBundle, Velocity}, event_loop::PacketEvent, player_list::{DisplayName, Listed, PlayerListEntryBundle}, prelude::*, protocol::packets::play::HandSwingC2s, spawn::IsFlat, CompressionThreshold, ServerSettings};

const START_POS: DVec3 = DVec3::new(0.0, 100.0, 0.0);
const VIEW_DIST: u8 = 10;
const GEN_DIST: i32 = 10;
const PUPPET_SPEED: f32 = 0.2;

#[derive(Component)]
struct GameState {
    puppet: Entity,
    sneaking: bool,
    coins: u32,
}

#[derive(Component)]
struct IsPuppet;

#[derive(Component)]
struct Owner(Entity);

pub fn main() {
    let Ok(config) = std::fs::read_to_string("config.json") else {
        eprintln!("Failed to read `config.json`. Exiting.");
        return;
    };
    let Ok(config) = json::parse(&config) else {
        eprintln!("Failed to parse `config.json`. Exiting.");
        return;
    };

    if config["server"].is_null() {
        eprintln!("`server` or `world` key is missing in `config.json`. Exiting.");
        return;
    }

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
        .add_systems(EventLoopUpdate, handle_interactions)
        .add_systems(
            Update,
            (
                init_clients,
                cleanup_clients,
                reset_clients.before(handle_movement),
                manage_chunks,
                manage_blocks,
                lock_look,
                handle_movement,
                apply_puppet_physics.after(handle_movement),
                despawn_disconnected_clients,
            ),
        )
        .run();
}

fn init_clients(
    mut clients: Query<
        (
            Entity,
            &mut Client,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut IsFlat,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    mut commands: Commands,
) {
    for (entity, mut client, mut visible_chunk_layer, mut visible_entity_layers, mut is_flat, mut game_mode) in clients.iter_mut() {
        visible_chunk_layer.0 = entity;
        visible_entity_layers.0.insert(entity);
        is_flat.0 = true;
        *game_mode = GameMode::Spectator;

        client.send_chat_message("Welcome to train chase!".italic());

        let npc_id = UniqueId::default();
        let npc_entity_id = commands.spawn(PlayerEntityBundle {
            layer: EntityLayerId(entity),
            uuid: npc_id,
            position: Position(START_POS + DVec3::new(0.5, 1.0, 0.5)),
            look: Look::new(0.0, 0.0),
            head_yaw: HeadYaw(0.0),
            ..Default::default()
        }).id();
        commands.entity(npc_entity_id).insert((IsPuppet, Owner(entity)));
        commands.spawn(PlayerListEntryBundle {
            uuid: npc_id,
            username: Username("Player".into()),
            display_name: DisplayName("Player".color(Color::RED).into()),
            listed: Listed(false),
            ..Default::default()
        });

        let state = GameState {
            puppet: npc_entity_id,
            sneaking: false,
            coins: 0,
        };

        let layer = ChunkLayer::new(ident!("overworld"), &dimensions, &biomes, &server);
        let entity_layer = EntityLayer::new(&server);

        commands.entity(entity).insert((state, layer, entity_layer));
    }
}

fn cleanup_clients(
    mut removed: RemovedComponents<Client>,
    clients: Query<&GameState>,
    puppets: Query<Entity, With<IsPuppet>>,
    mut commands: Commands,
) {
    for entity in removed.read() {
        if let Ok(state) = clients.get(entity) {
            if let Ok(puppet) = puppets.get(state.puppet) {
                commands.entity(puppet).insert(Despawned);
            }
        }
    }
}

fn reset_clients(
    mut clients: Query<(
        &mut Client,
        &mut Position,
        &mut GameState,
        &mut ChunkLayer,
    )>,
    mut puppets: Query<(&mut Position, &mut Velocity, &Owner), (With<IsPuppet>, Without<Client>)>,
) {
    for (mut puppet_pos, mut puppet_vel, owner) in puppets.iter_mut() {
        if let Ok((mut client, mut pos, mut state, mut layer)) = clients.get_mut(owner.0) {
            let block = layer.block(BlockPos::from(puppet_pos.0.floor()));
            let touched_block = block.is_some() && block.unwrap().state == BlockState::OAK_SIGN;
            let out_of_bounds = puppet_pos.0.y < START_POS.y - 32_f64;

            if out_of_bounds || touched_block || state.is_added() {
                if touched_block && !state.is_added() {
                    client.send_chat_message(
                        "You got ".italic()
                            + state
                                .coins
                                .to_string()
                                .color(Color::GOLD)
                                .bold()
                                .not_italic()
                            + " coins!".italic(),
                    );
                }

                for pos in ChunkView::new(START_POS.into(), VIEW_DIST).iter() {
                    layer.insert_chunk(pos, UnloadedChunk::new());
                }

                for i in -1..=1 {
                    for j in 0..GEN_DIST {
                        layer.set_block(BlockPos::new(START_POS.x as i32 + i, START_POS.y as i32, START_POS.z as i32 + j), BlockState::DIRT);
                        layer.set_block(BlockPos::new(START_POS.x as i32 + i, START_POS.y as i32 + 1, START_POS.z as i32 + j), BlockState::RAIL);
                    }
                }

                state.coins = 0;

                puppet_vel.0 = Vec3::ZERO;
                puppet_pos.set([
                    f64::from(START_POS.x) + 0.5,
                    f64::from(START_POS.y) + 1.0,
                    f64::from(START_POS.z) + 0.5,
                ]);

                pos.0 = puppet_pos.get() + DVec3::new(0.0, 4.0, -4.0);
            }
        }
    }
}

fn manage_chunks(mut clients: Query<(&Position, &OldPosition, &mut ChunkLayer), With<Client>>) {
    for (pos, old_pos, mut layer) in clients.iter_mut() {
        let old_view = ChunkView::new(old_pos.get().into(), VIEW_DIST);
        let view = ChunkView::new(pos.0.into(), VIEW_DIST);

        if old_view != view {
            for pos in old_view.diff(view) {
                layer.remove_chunk(pos);
            }

            for pos in view.diff(old_view) {
                layer.chunk_entry(pos).or_default();
            }
        }
    }
}

fn manage_blocks(
    mut clients: Query<&mut ChunkLayer, With<Client>>,
    puppets: Query<(&Position, &Owner), (With<IsPuppet>, Without<Client>)>,
) {
    for (puppet_pos, owner) in puppets.iter() {
        if let Ok(mut layer) = clients.get_mut(owner.0) {
            match layer.block(BlockPos::new(puppet_pos.0.x.floor() as i32, START_POS.y as i32, puppet_pos.0.z.floor() as i32 + GEN_DIST)) {
                Some(block) => {
                    if block.state == BlockState::AIR {
                        for i in -1..=1 {
                            layer.set_block(BlockPos::new(START_POS.x as i32 + i, START_POS.y as i32, puppet_pos.0.z.floor() as i32 + GEN_DIST), BlockState::DIRT);
                            if fastrand::u8(0..20) == 0 {
                                layer.set_block(BlockPos::new(START_POS.x as i32 + i, START_POS.y as i32 + 1, puppet_pos.0.z.floor() as i32 + GEN_DIST), BlockState::OAK_SIGN);
                            } else {
                                layer.set_block(BlockPos::new(START_POS.x as i32 + i, START_POS.y as i32 + 1, puppet_pos.0.z.floor() as i32 + GEN_DIST), BlockState::RAIL);
                            }
                        }
                    }
                }
                None => {}
            }
        }
    }
}

fn lock_look(
    mut clients: Query<&mut Look, With<Client>>
) {
    for mut look in clients.iter_mut() {
        look.yaw = 0.0;
        look.pitch = 40.0;
    }
}

fn handle_interactions(
    clients: Query<&GameState>,
    mut puppets: Query<&mut Velocity, With<IsPuppet>>,
    mut packets: EventReader<PacketEvent>,
) {
    for packet in packets.read() {
        if let Some(_) = packet.decode::<HandSwingC2s>() {
            if let Ok(state) = clients.get(packet.client) {
                if let Ok(mut vel) = puppets.get_mut(state.puppet) {
                    if vel.0.y == 0.0 {
                        vel.0.y += 0.5;
                    }
                }
            }
        }
    }
}

fn handle_movement(
    mut clients: Query<(&mut Client, &Position, &OldPosition, &mut GameState), With<Client>>,
    mut puppets: Query<(&Position, &mut Velocity), (With<IsPuppet>, Without<Client>)>,
    mut sneaking: EventReader<SneakEvent>,
) {
    for event in sneaking.read() {
        if let Ok((_, _, _, mut state)) = clients.get_mut(event.client) {
            state.sneaking = event.state == SneakState::Start;
        }
    }
    for (mut client, pos, old_pos, state) in clients.iter_mut() {
        if let Ok((puppet_pos, mut puppet_vel)) = puppets.get_mut(state.puppet) {
            if pos.0 != puppet_pos.get() + DVec3::new(0.0, 4.0, -4.0) && (pos.0*100.0).round()/100.0 != (old_pos.get()*100.0).round()/100.0 {
                let vel = Vec3::new((pos.0.x - old_pos.get().x) as f32, (pos.0.y - old_pos.get().y) as f32, (pos.0.z - old_pos.get().z) as f32) * 2.0;
                if vel.x != 0.0 {
                    puppet_vel.0.x = vel.x;
                }
            }

            let mut vel = Vec3::ZERO;

            vel.y = if pos.0.y - START_POS.y < 4.1 || pos.0.y - START_POS.y > 4.1 {
                (START_POS.y + 4.0 - pos.0.y) as f32
            } else {
                0.0
            };
            vel.z = if puppet_pos.0.z - pos.0.z < 4.1 || puppet_pos.0.z - pos.0.z > 4.1 {
                (puppet_pos.0.z - 4.0 - pos.0.z) as f32 * PUPPET_SPEED * 100.0
            } else {
                PUPPET_SPEED * 20.0
            };

            client.set_velocity(vel);
        };
    }
}

fn apply_puppet_physics(
    mut clients: Query<&ChunkLayer, With<Client>>,
    mut puppets: Query<(&mut Position, &mut Velocity, &Owner), (With<IsPuppet>, Without<Client>)>,
) {
    for (mut puppet_pos, mut vel, owner) in puppets.iter_mut() {
        if let Ok(layer) = clients.get_mut(owner.0) {
            vel.0.y -= 0.05;
            let block = layer.block(BlockPos::from(puppet_pos.0 + DVec3::from(vel.0)));
            if block.is_some() && block.unwrap().state == BlockState::DIRT {
                vel.0.y = 0.0;
            }
            puppet_pos.0 += DVec3::from(vel.0);
            if vel.0.x != 0.0 {
                vel.0.x -= vel.0.x*0.1;
            }
            vel.0.z = PUPPET_SPEED;
        }
    }
}