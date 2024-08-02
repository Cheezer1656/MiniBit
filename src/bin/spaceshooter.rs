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

#[path = "../lib/mod.rs"]
mod lib;


use std::marker::PhantomData;

use lib::config::{ConfigLoaderPlugin, EmptyConfig};
use valence::{
    entity::{
        entity::NoGravity, falling_block::{FallingBlockEntity, FallingBlockEntityBundle}, ObjectData, Velocity
    }, event_loop::PacketEvent, prelude::*, protocol::{packets::play::HandSwingC2s, sound::SoundCategory, Sound}, spawn::IsFlat
};

const START_POS: DVec3 = DVec3::new(0.0, 100.0, 0.0);
const VIEW_DIST: u8 = 10;

#[derive(Component, Default)]
struct GameState {
    blocks: Vec<Entity>,
    score: u32,
}

fn main() {
    App::new()
        .add_plugins(ConfigLoaderPlugin::<EmptyConfig> { phantom: PhantomData })
        .add_plugins(DefaultPlugins)
        .add_systems(
            Update,
            (
                init_clients,
                spawn_blocks,
                move_blocks,
                shoot,
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
            &mut Position,
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
        mut client,
        mut pos,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut is_flat,
        mut game_mode,
    ) in &mut clients
    {
        pos.0 = START_POS + DVec3::new(0.5, 0.0, 0.5);
        visible_chunk_layer.0 = entity;
        visible_entity_layers.0.insert(entity);
        is_flat.0 = true;
        *game_mode = GameMode::Creative;

        client.send_chat_message("Welcome to MiniBit's Space Shooter!".italic());

        let mut layer = ChunkLayer::new(ident!("the_end"), &dimensions, &biomes, &server);
        let entity_layer = EntityLayer::new(&server);

        for pos in ChunkView::new(START_POS.into(), VIEW_DIST).iter() {
            layer.insert_chunk(pos, UnloadedChunk::new());
        }

        layer.set_block(
            BlockPos::from(START_POS - DVec3::new(0.0, 1.0, 0.0)),
            BlockState::OBSIDIAN,
        );

        commands
            .entity(entity)
            .insert((layer, entity_layer, GameState::default()));
    }
}

fn spawn_blocks(mut clients: Query<(Entity, &mut GameState), With<EntityLayer>>, mut commands: Commands) {
    for (layer, mut state) in clients.iter_mut() {
        if fastrand::u8(0..20) == 0 {
            let block_id = commands.spawn(FallingBlockEntityBundle {
                position: Position(START_POS + DVec3::new(fastrand::f64() * 40.0 - 20.0, fastrand::f64() * 40.0 - 20.0, fastrand::f64() * 10.0 + 20.0)),
                layer: EntityLayerId(layer),
                object_data: ObjectData(14),
                entity_no_gravity: NoGravity(true),
                velocity: Velocity(Vec3::new(0.0, 0.0, fastrand::f32() * 0.5)),
                ..Default::default()
            }).id();
            state.blocks.push(block_id);
        }
    }
}

fn move_blocks(mut falling_blocks: Query<(Entity, &mut Position, &Velocity), With<FallingBlockEntity>>, mut commands: Commands) {
    for (entity, mut pos, vel) in falling_blocks.iter_mut() {
        pos.0.z -= vel.0.z as f64;
        if pos.0.z < -10.0 {
            commands.entity(entity).insert(Despawned);
        }
    }
}

fn shoot(
    mut clients: Query<(&mut Client, &Position, &Look, &mut GameState)>,
    falling_blocks: Query<(Entity, &Position, &EntityLayerId), With<FallingBlockEntity>>,
    mut packets: EventReader<PacketEvent>,
    mut commands: Commands
) {
    for pkt in packets.read() {
        if let Some(_) = pkt.decode::<HandSwingC2s>() {
            if let Ok((mut client, player_pos, look, mut state)) = clients.get_mut(pkt.client) {
                let yaw = look.yaw.to_radians() as f64;
                let pitch = look.pitch.to_radians() as f64;
                let direction = DVec3::new(
                    -yaw.sin() * pitch.cos(),
                    -pitch.sin(),
                    yaw.cos() * pitch.cos(),
                ) * 0.99;

                let mut pos = player_pos.0 + DVec3::new(0.0, 1.6, 0.0);
                for _ in 0..100 {
                    pos += direction;
                    client.play_particle(&Particle::Dust { rgb: Vec3::new(255.0, 0.0, 0.0), scale: 1.0 }, true, pos, Vec3::splat(0.001), 0.01, 2);
                    let mut hit_blocks: Vec<usize> = Vec::new();
                    for (i, entity) in state.blocks.iter().enumerate() {
                        if let Ok((entity, block_pos, block_layer)) = falling_blocks.get(*entity) {
                            if block_layer.0 == pkt.client && (block_pos.0 - pos).length() < 1.0 {
                                client.play_sound(Sound::EntityArrowHitPlayer, SoundCategory::Master, player_pos.0, 1.0, 1.0);
                                commands.entity(entity).insert(Despawned);
                                hit_blocks.push(i);
                                state.score += 1;
                                client.set_action_bar(&format!("Score: {}", state.score).color(Color::GREEN).bold());
                                break;
                            }
                        }
                    }
                    for i in hit_blocks.iter() {
                        state.blocks.remove(*i);
                    }
                }
            }
        }
    }
}