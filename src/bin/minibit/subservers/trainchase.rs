/*
    MiniBit - A Minecraft minigame server network written in Rust.
    Copyright (C) 2024  Cheezer1656 (https://github.com/Cheezer1656/)

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published
    by the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

#![allow(clippy::type_complexity)]

use std::{
    marker::PhantomData,
    time::{Duration, Instant},
};
use std::path::PathBuf;
use minibit_lib::config::{ConfigLoaderPlugin, EmptyConfig};
use valence::{
    entity::{
        entity::{self, NoGravity},
        item::{ItemEntity, ItemEntityBundle, Stack},
        player::{PlayerEntityBundle, PlayerModelParts},
        EntityId, Pose, Velocity,
    },
    event_loop::PacketEvent,
    math::{IVec3, Vec3Swizzles},
    player_list::{DisplayName, Listed, PlayerListEntryBundle},
    prelude::*,
    protocol::{
        packets::play::{
            entity_equipment_update_s2c::EquipmentEntry, EntityAnimationS2c,
            EntityEquipmentUpdateS2c, HandSwingC2s,
        },
        sound::SoundCategory,
        Sound, VarInt, WritePacket,
    },
    spawn::IsFlat,
};

const START_POS: DVec3 = DVec3::new(0.0, 100.0, 0.0);
const VIEW_DIST: u8 = 10;
const GEN_DIST: i32 = 15;
const WALL_HEIGHT: i32 = 10;
const PUPPET_SPEED: f32 = 0.2;

#[derive(Resource)]
struct Tick(u64);

#[derive(Component)]
struct GameStarted;

#[derive(Component)]
struct GameState {
    start: u64,
    puppet: Entity,
    cop: Entity,
    sneaking: bool,
    coins: u32,
}

#[derive(Component)]
struct DuckingState {
    time: Option<Instant>,
}

#[derive(Component)]
struct IsPuppet;

#[derive(Component)]
struct IsCop;

#[derive(Component)]
struct Owner(Entity);

pub fn main(path: PathBuf) {
    App::new()
        .add_plugins(ConfigLoaderPlugin::<EmptyConfig> {
            path,
            phantom: PhantomData,
        })
        .add_plugins(DefaultPlugins)
        .insert_resource(Tick(0))
        .add_systems(EventLoopUpdate, handle_interactions)
        .add_systems(
            Update,
            (
                increment_tick,
                init_clients,
                cleanup_clients,
                reset_clients.before(handle_movement),
                play_animation.after(increment_tick).after(init_clients),
                manage_chunks,
                manage_blocks,
                lock_look,
                handle_movement,
                apply_physics.after(handle_movement),
                check_for_coins,
                stop_ducking,
                despawn_disconnected_clients,
            ),
        )
        .run();
}

fn increment_tick(mut tick: ResMut<Tick>) {
    tick.0 += 1;
}

fn init_clients(
    tick: Res<Tick>,
    mut clients: Query<
        (
            Entity,
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
    for (
        entity,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut is_flat,
        mut game_mode,
    ) in clients.iter_mut()
    {
        visible_chunk_layer.0 = entity;
        visible_entity_layers.0.insert(entity);
        is_flat.0 = true;
        *game_mode = GameMode::Spectator;

        let puppet_id = UniqueId::default();
        let puppet_entity_id = commands
            .spawn(PlayerEntityBundle {
                layer: EntityLayerId(entity),
                uuid: puppet_id,
                position: Position(START_POS + DVec3::new(0.0, 1.0, 0.5)),
                look: Look::new(90.0, 0.0),
                head_yaw: HeadYaw(90.0),
                player_player_model_parts: PlayerModelParts(126),
                ..Default::default()
            })
            .id();
        commands.entity(puppet_entity_id).insert((
            IsPuppet,
            Owner(entity),
            DuckingState { time: None },
        ));
        let mut puppet_props = Properties::default();
        puppet_props.set_skin("ewogICJ0aW1lc3RhbXAiIDogMTcyMDg5Mzk0ODQzMSwKICAicHJvZmlsZUlkIiA6ICI0OTY5YTVlZTYxMTY0MDBkYTM4YzhmZjRiMWJhZTZiZiIsCiAgInByb2ZpbGVOYW1lIiA6ICJSZWFjdFpJUCIsCiAgInNpZ25hdHVyZVJlcXVpcmVkIiA6IHRydWUsCiAgInRleHR1cmVzIiA6IHsKICAgICJTS0lOIiA6IHsKICAgICAgInVybCIgOiAiaHR0cDovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS8yMzc4NzYzYzY3Mjg5MzllMWI0MDc5OWJjNDY5NWYxZDA4OGRjYzFkOWFhZDQxZWI4MDNjNzVkNDIwYmExZjk1IgogICAgfQogIH0KfQ==", "ax1Jq5CfbvonOQ2xP1wk2dyORpDavqhCvwrhdWblg7AvbthDlyNUHO6mWSSGMZwqHL+2A40DnUEcKsvMJhvjpP4QYUGowv0uCWPO8IemFXdrapZvprIi+TcBBP+FAI55cABR2SuanlBFs2azvT6wBdiBoASFCYr+7IZXhjVZct2siXprwXT0xEVDCw5Zy8mMc23iItDGxjzrNrA2/we6Hfapg+NUUu4xW2tm6SSkeSQi1Ox+TH9H4Z8rLUDv/4w1NB9bZuleS/X/HGHSs1BuS9XzCYuTmzkg9D1CtEVVFv0QgSw6Z7LdrOpls30iMaqbgJbhMUWF2L03gySiQlZEKzKw99SCxmLi9DopOfEBQzPQ2fHwyogjPA/BF7S0jbipZEYv5bcHi9hmjBeEJpRkQWaiJVGpg73btnzBZQHDES64wiNIQrNnKYgT77ClqG+3tfFvfBr44iEcwc+HJjMSZZRak1UsG5e7h7ki0JMV5klHacnvbEV06iW9y4RiO6v4hacMtBixCbVC0ZwGys1uQrSSoW1KJMZYNEW2qarePDGv2XHaJoCRXSnFxMmYPd1CH8q+N/hd5QBK/fXenhYodgYWwHxFhuV0WoI/43dtv7szoudNzm+6Q4piQtLdnl9VrGuLFZaSO0euephdp/Uqq+HnwRdd5Ve/wDqEaepZjsc=");
        commands.spawn(PlayerListEntryBundle {
            uuid: puppet_id,
            username: Username("Jake".into()),
            display_name: DisplayName("Jake".color(Color::RED).into()),
            listed: Listed(false),
            properties: puppet_props,
            ..Default::default()
        });

        let cop_id = UniqueId::default();
        let cop_entity_id = commands
            .spawn(PlayerEntityBundle {
                layer: EntityLayerId(entity),
                uuid: cop_id,
                position: Position(START_POS + DVec3::new(0.5, 1.0, -6.5)),
                look: Look::new(0.0, 0.0),
                head_yaw: HeadYaw(0.0),
                player_player_model_parts: PlayerModelParts(126),
                ..Default::default()
            })
            .id();
        commands
            .entity(cop_entity_id)
            .insert((IsCop, Owner(entity)));
        let mut cop_props = Properties::default();
        cop_props.set_skin(
            "ewogICJ0aW1lc3RhbXAiIDogMTcyMDU5MDYxODAyOSwKICAicHJvZmlsZUlkIiA6ICJmODg2ZDI3YjhjNzU0NjAyODYyYTM1M2NlYmYwZTgwZiIsCiAgInByb2ZpbGVOYW1lIiA6ICJOb2JpbkdaIiwKICAic2lnbmF0dXJlUmVxdWlyZWQiIDogdHJ1ZSwKICAidGV4dHVyZXMiIDogewogICAgIlNLSU4iIDogewogICAgICAidXJsIiA6ICJodHRwOi8vdGV4dHVyZXMubWluZWNyYWZ0Lm5ldC90ZXh0dXJlL2EwMWU3ZmVhMTRhYjdmNWZhODEzOTY5ZWU2OGI1MmE5YTgzZWI2ODdlN2UwMjEwZDViN2MwOGNmYzYxMDZmOTIiCiAgICB9CiAgfQp9",
                "jNqgs70maccFU7G4tzmaabM9KI8NhopNtNswHArG2qONucEYLmNwt6TaE1Cr7AXgE2w4OmtZ2/Ov8Lp9YrXomAC74lw58tSw0R6CMdUn/uf5Sz0ByMfspLvoiv23OVewuj76HToNcEAbhMNTeyJbI2ucALohSFY7Z4/iwgf+0OP5qH8YpwyxTtREOlfz/Jfwkn4UgSV+yHTAUoArp7zgmsUQoHdScfrwbm61oPaXWd+kTZgsJKr3FlJwpvd2lJi8fay00O3LY03Dhqz//VWFmRmPnRg4PswF3ATRLxvxc0C594bvsVhp6UuvSiFPJyBGamjTPK552X3rgrN8F1rXgT7U7/p6wH2WJvlIH9XWMc+7xnfatmsXrDqIWDJxdzl1ZRxM144L2GX7YX1gzUg4OLtV/HXCftfpw0cQHq+cjLQ/qDT6laNCxzMiWOEXyj6J450Aph7Bj7IIFDF+Ak+gk34YoGvSzBI0poVqqNPb4Q08T+/5SM32sqg75I0QuIcdJlzprtE4JlmWBUzX4714MNVyH+XWhVyghaX5mEh9sSFZaJfo+ulkGsggotUJL9gEZehF9PdqKySekmWVCG1tVs5/XR13jMhsnPk2Luty+4DVwjOkQnl5jQpdq5SPtEc+D55PJ5MWcy3t3m0kRbRKUf6tZZOOwj26FcO+Llw442I=",
        );
        commands.spawn(PlayerListEntryBundle {
            uuid: cop_id,
            username: Username("Inspector".into()),
            display_name: DisplayName("Inspector".color(Color::RED).into()),
            listed: Listed(false),
            properties: cop_props,
            ..Default::default()
        });

        let state = GameState {
            start: tick.0,
            puppet: puppet_entity_id,
            cop: cop_entity_id,
            sneaking: false,
            coins: 0,
        };

        let mut layer = ChunkLayer::new(ident!("overworld"), &dimensions, &biomes, &server);
        let entity_layer = EntityLayer::new(&server);

        for pos in ChunkView::new(START_POS.into(), VIEW_DIST).iter() {
            layer.insert_chunk(pos, UnloadedChunk::new());
        }

        commands.entity(entity).insert((state, layer, entity_layer));
    }
}

fn cleanup_clients(
    mut removed: RemovedComponents<Client>,
    clients: Query<&GameState>,
    puppets: Query<Entity, With<IsPuppet>>,
    cops: Query<Entity, With<IsCop>>,
    mut commands: Commands,
) {
    for entity in removed.read() {
        if let Ok(state) = clients.get(entity) {
            if let Ok(puppet) = puppets.get(state.puppet) {
                commands.entity(puppet).insert(Despawned);
            }
            if let Ok(cop) = cops.get(state.cop) {
                commands.entity(cop).insert(Despawned);
            }
        }
    }
}

fn reset_clients(
    mut clients: Query<(
        &mut Client,
        &mut Position,
        &mut GameState,
        &mut GameStarted,
        &mut ChunkLayer,
    )>,
    mut puppets: Query<
        (
            &mut Position,
            &mut Velocity,
            &mut Look,
            &mut HeadYaw,
            &DuckingState,
            &Owner,
        ),
        (With<IsPuppet>, Without<Client>),
    >,
    mut cops: Query<
        (&mut Position, &mut Velocity),
        (With<IsCop>, Without<Client>, Without<IsPuppet>),
    >,
) {
    for (mut puppet_pos, mut puppet_vel, mut puppet_look, mut puppet_yaw, ducking, owner) in
        puppets.iter_mut()
    {
        if let Ok((mut client, mut pos, mut state, started, mut layer)) = clients.get_mut(owner.0) {
            let block1 = layer.block(BlockPos::from(
                puppet_pos.0.floor() + DVec3::new(0.0, 1.0, 0.0),
            ));
            let touched_block1 = if let Some(block) = block1 {
                block.state != BlockState::AIR && block.state != BlockState::RAIL
            } else {
                false
            };
            let block2 = layer.block(BlockPos::from(
                puppet_pos.0.floor() + DVec3::new(0.0, 2.0, 0.0),
            ));
            let touched_block2 = if let Some(block) = block2 {
                block.state != BlockState::AIR && block.state != BlockState::RAIL
            } else {
                false
            };
            let out_of_bounds = puppet_pos.0.y < START_POS.y - 32_f64;

            if out_of_bounds
                || touched_block1
                || (touched_block2 && ducking.time.is_none())
                || started.is_added()
            {
                if touched_block1 || (touched_block2 && ducking.time.is_none()) {
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
                    client.send_chat_message(
                        "Distance traveled: ".italic()
                            + format!("{:.2}", pos.0.z - START_POS.z)
                                .color(Color::GREEN)
                                .bold()
                                .not_italic()
                            + " blocks".italic(),
                    );
                    for pos in ChunkView::new(START_POS.into(), VIEW_DIST).iter() {
                        layer.insert_chunk(pos, UnloadedChunk::new());
                    }
                    gen_starting_blocks(&mut layer, 3);
                }

                state.coins = 0;

                puppet_look.yaw = 0.0;
                puppet_yaw.0 = 0.0;
                puppet_vel.0 = Vec3::new(0.0, 0.0, PUPPET_SPEED);
                puppet_pos.set([
                    f64::from(START_POS.x) + 0.5,
                    f64::from(START_POS.y) + 1.0,
                    f64::from(START_POS.z) + 0.5,
                ]);

                pos.0 = puppet_pos.get() + DVec3::new(0.0, 4.0, -4.0);

                if let Ok((mut cop_pos, mut cop_vel)) = cops.get_mut(state.cop) {
                    cop_vel.0 = Vec3::new(0.0, 0.0, PUPPET_SPEED);
                    cop_pos.0 = puppet_pos.get() + DVec3::new(0.0, 0.0, -3.0);
                }
            }
        }
    }
}

fn gen_starting_blocks(layer: &mut ChunkLayer, blocks_behind: u8) {
    let blocks_behind = -(blocks_behind as i32);
    for i in -1..=1 {
        for j in blocks_behind..GEN_DIST {
            layer.set_block(
                BlockPos::new(
                    START_POS.x as i32 + i,
                    START_POS.y as i32,
                    START_POS.z as i32 + j,
                ),
                BlockState::DIRT,
            );
            layer.set_block(
                BlockPos::new(
                    START_POS.x as i32 + i,
                    START_POS.y as i32 + 1,
                    START_POS.z as i32 + j,
                ),
                BlockState::RAIL,
            );
        }
    }
    for i in blocks_behind..GEN_DIST {
        for j in 0..WALL_HEIGHT {
            layer.set_block(
                BlockPos::new(
                    START_POS.x as i32 - 2,
                    START_POS.y as i32 + j,
                    START_POS.z as i32 + i,
                ),
                BlockState::STONE_BRICKS,
            );
        }
    }
    for i in blocks_behind..GEN_DIST {
        for j in 0..WALL_HEIGHT {
            layer.set_block(
                BlockPos::new(
                    START_POS.x as i32 + 2,
                    START_POS.y as i32 + j,
                    START_POS.z as i32 + i,
                ),
                BlockState::STONE_BRICKS,
            );
        }
    }
}

fn play_animation(
    tick: Res<Tick>,
    mut clients: Query<
        (
            Entity,
            &mut Client,
            &mut Position,
            &mut Look,
            &mut ChunkLayer,
            &GameState,
        ),
        Without<GameStarted>,
    >,
    mut puppets: Query<(&EntityId, &mut Position, &mut HeadYaw), (With<IsPuppet>, Without<Client>)>,
    mut cops: Query<(&EntityId, &mut Position), (With<IsCop>, Without<IsPuppet>, Without<Client>)>,
    mut commands: Commands,
) {
    for (entity, mut client, mut pos, mut look, mut layer, state) in clients.iter_mut() {
        let Ok((puppet_id, _puppet_pos, mut puppet_yaw)) = puppets.get_mut(state.puppet) else {
            continue;
        };
        let Ok((cop_id, mut cop_pos)) = cops.get_mut(state.cop) else {
            continue;
        };
        match tick.0 - state.start {
            2 => {
                gen_starting_blocks(&mut layer, 10);
                pos.0 = START_POS + DVec3::new(1.75, 1.0, 1.0);
                look.yaw = 145.0;
                look.pitch = 20.0;
                client.write_packet(&EntityEquipmentUpdateS2c {
                    entity_id: VarInt(puppet_id.get()),
                    equipment: vec![EquipmentEntry {
                        slot: 0,
                        item: ItemStack::new(ItemKind::Potion, 1, None),
                    }],
                });
            }
            4 | 10 | 16 | 24 | 30 | 36 => {
                client.write_packet(&EntityAnimationS2c {
                    entity_id: VarInt(puppet_id.get()),
                    animation: 0,
                });
            }
            37..=49 => {
                puppet_yaw.0 += 3.0;
            }
            50 => {
                client.write_packet(&EntityEquipmentUpdateS2c {
                    entity_id: VarInt(puppet_id.get()),
                    equipment: vec![EquipmentEntry {
                        slot: 0,
                        item: ItemStack::EMPTY,
                    }],
                });
            }
            30..=69 => {
                cop_pos.0.z += 0.2;
            }
            70 => {
                client.send_chat_message("Inspector: STOP!".color(Color::RED).bold());
                client.write_packet(&EntityAnimationS2c {
                    entity_id: VarInt(cop_id.get()),
                    animation: 0,
                });
            }
            100 => {
                commands.entity(entity).insert(GameStarted);
            }
            _ => {}
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
    puppets: Query<(&Position, &EntityLayerId, &Owner), (With<IsPuppet>, Without<Client>)>,
    mut commands: Commands,
) {
    for (puppet_pos, entity_layer, owner) in puppets.iter() {
        if let Ok(mut layer) = clients.get_mut(owner.0) {
            match layer.block(BlockPos::new(
                puppet_pos.0.x.floor() as i32,
                START_POS.y as i32,
                puppet_pos.0.z.floor() as i32 + GEN_DIST,
            )) {
                Some(block) => {
                    if block.state == BlockState::AIR {
                        for i in 0..WALL_HEIGHT {
                            layer.set_block(
                                BlockPos::new(
                                    START_POS.x as i32 + 2,
                                    START_POS.y as i32 + i,
                                    puppet_pos.0.z as i32 + GEN_DIST,
                                ),
                                BlockState::STONE_BRICKS,
                            );
                        }
                        for i in 0..WALL_HEIGHT {
                            layer.set_block(
                                BlockPos::new(
                                    START_POS.x as i32 - 2,
                                    START_POS.y as i32 + i,
                                    puppet_pos.0.z as i32 + GEN_DIST,
                                ),
                                BlockState::STONE_BRICKS,
                            );
                        }
                        for i in -1..=1 {
                            let block_pos = BlockPos::new(
                                START_POS.x as i32 + i,
                                START_POS.y as i32,
                                puppet_pos.0.z.floor() as i32 + GEN_DIST,
                            );
                            layer.set_block(block_pos, BlockState::DIRT);
                            match fastrand::u8(0..60) {
                                0..=2 => {
                                    layer.set_block(
                                        block_pos + IVec3::new(0, 1, 0),
                                        BlockState::OAK_SIGN,
                                    );
                                }
                                3..=4 => {
                                    layer.set_block(
                                        block_pos + IVec3::new(0, 2, 0),
                                        BlockState::OAK_SIGN,
                                    );
                                }
                                5 => {
                                    for i in 0..fastrand::u8(3..=5) {
                                        layer.set_block(
                                            block_pos + IVec3::new(0, 1, -(i as i32)),
                                            BlockState::GRAY_CONCRETE,
                                        );
                                        layer.set_block(
                                            block_pos + IVec3::new(0, 2, -(i as i32)),
                                            BlockState::LIGHT_GRAY_CONCRETE,
                                        );
                                    }
                                }
                                _ => {
                                    layer.set_block(
                                        block_pos + IVec3::new(0, 1, 0),
                                        BlockState::RAIL,
                                    );
                                }
                            };
                            if fastrand::u8(0..10) == 0 {
                                commands.spawn(ItemEntityBundle {
                                    item_stack: Stack(ItemStack::new(ItemKind::GoldBlock, 1, None)),
                                    position: Position(DVec3::new(
                                        block_pos.x as f64,
                                        block_pos.y as f64 + fastrand::u8(1..=3) as f64,
                                        block_pos.z as f64,
                                    )),
                                    velocity: Velocity(Vec3::ZERO),
                                    entity_no_gravity: NoGravity(true),
                                    layer: *entity_layer,
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
                None => {}
            }
        }
    }
}

fn lock_look(mut clients: Query<&mut Look, (With<Client>, With<GameStarted>)>) {
    for mut look in clients.iter_mut() {
        look.yaw = 0.0;
        look.pitch = 40.0;
    }
}

fn handle_interactions(
    clients: Query<&GameState, With<GameStarted>>,
    mut puppets: Query<(&mut DuckingState, &mut entity::Pose), With<IsPuppet>>,
    mut packets: EventReader<PacketEvent>,
) {
    for packet in packets.read() {
        if let Some(_) = packet.decode::<HandSwingC2s>() {
            if let Ok(state) = clients.get(packet.client) {
                if let Ok((mut ducking, mut pose)) = puppets.get_mut(state.puppet) {
                    pose.0 = Pose::Swimming;
                    ducking.time = Some(Instant::now());
                }
            }
        }
    }
}

fn handle_movement(
    mut clients: Query<(&mut Client, &Position, &OldPosition, &mut GameState), With<GameStarted>>,
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
            if pos.0 != puppet_pos.get() + DVec3::new(0.0, 4.0, -4.0)
                && (pos.0 * 100.0).round() / 100.0 != (old_pos.get() * 100.0).round() / 100.0
            {
                let vel = Vec3::new(
                    (pos.0.x - old_pos.get().x) as f32,
                    (pos.0.y - old_pos.get().y) as f32,
                    (pos.0.z - old_pos.get().z) as f32,
                ) * 2.0;
                if vel.x != 0.0 {
                    puppet_vel.0.x = vel.x;
                }
                if vel.y > 0.1 && puppet_vel.0.y == 0.0 {
                    puppet_vel.0.y = 0.5;
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

fn apply_physics(
    clients: Query<&ChunkLayer, (With<Client>, With<GameStarted>)>,
    mut npcs: Query<
        (&mut Position, &mut Velocity, &Owner),
        (Or<(With<IsPuppet>, With<IsCop>)>, Without<Client>),
    >,
) {
    for (mut puppet_pos, mut vel, owner) in npcs.iter_mut() {
        if let Ok(layer) = clients.get(owner.0) {
            let block = layer.block(BlockPos::from(puppet_pos.0 + DVec3::from(vel.0)));
            vel.0.y = if let Some(block) = block {
                if block.state != BlockState::AIR
                    && block.state != BlockState::RAIL
                    && block.state != BlockState::OAK_SIGN
                {
                    0.0
                } else {
                    vel.0.y - 0.05
                }
            } else {
                vel.0.y - 0.05
            };
            if vel.0.x != 0.0 {
                vel.0.x -= vel.0.x * 0.1;
            }
            if puppet_pos.x > 1.5 {
                puppet_pos.0.x = 1.5;
            } else if puppet_pos.x < -0.5 {
                puppet_pos.0.x = -0.5;
            }
            puppet_pos.0 += DVec3::from(vel.0);
        }
    }
}

fn check_for_coins(
    mut clients: Query<(&mut Client, &mut GameState), With<Client>>,
    puppets: Query<(&Position, &Owner), With<IsPuppet>>,
    items: Query<(Entity, &Position, &Stack), With<ItemEntity>>,
    mut commands: Commands,
) {
    for (pos, owner) in puppets.iter() {
        if let Ok((mut client, mut state)) = clients.get_mut(owner.0) {
            for (entity, item_pos, stack) in items.iter() {
                let diff = item_pos.0 - pos.0;
                if diff.xz().length() < 1.0 && diff.y < 1.8 && diff.y > 0.0 {
                    client.play_sound(
                        Sound::EntityArrowHitPlayer,
                        SoundCategory::Master,
                        pos.0,
                        1.0,
                        1.0,
                    );
                    state.coins += stack.0.count as u32;
                    commands.entity(entity).insert(Despawned);
                }
            }
        }
    }
}

fn stop_ducking(mut puppets: Query<(&mut DuckingState, &mut entity::Pose), With<IsPuppet>>) {
    for (mut ducking, mut pose) in puppets.iter_mut() {
        if let Some(time) = ducking.time {
            if time.elapsed() > Duration::from_millis(750) {
                pose.0 = Pose::Standing;
                ducking.time = None;
            }
        }
    }
}
