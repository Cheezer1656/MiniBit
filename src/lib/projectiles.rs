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

#![allow(dead_code)]

use bevy_ecs::query::QueryData;
use parry3d::{math::Vector, na::{self, Isometry3}, query::{cast_shapes, ShapeCastOptions}, shape::Cuboid};
use valence::{entity::{arrow::{ArrowEntity, ArrowEntityBundle}, Velocity}, event_loop::PacketEvent, interact_item::InteractItemEvent, inventory::{HeldItem, PlayerAction}, prelude::*, protocol::{packets::play::PlayerActionC2s, sound::SoundCategory, Sound}};

#[derive(Component)]
struct BowDrawTick(pub i64);

#[derive(Component)]
pub struct ProjectileOwner(pub Entity);

#[derive(Event)]
pub struct ProjectileCollisionEvent {
    pub arrow: Entity,
    pub player: Entity,
}

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<ProjectileCollisionEvent>()
            .add_systems(EventLoopUpdate, (set_use_tick, handle_player_actions))
            .add_systems(Update, (init_clients, apply_arrow_physics, cleanup_arrows));
    }
}

fn init_clients(clients: Query<Entity, Added<Client>>, mut commands: Commands) {
    for entity in clients.iter() {
        commands.entity(entity).insert(BowDrawTick(i64::MAX));
    }
}

fn set_use_tick(
    mut clients: Query<(&Inventory, &HeldItem, &mut BowDrawTick), With<Client>>,
    mut events: EventReader<InteractItemEvent>,
    server: Res<Server>,
) {
    for event in events.read() {
        if let Ok((inv, held_item, mut draw_tick)) = clients.get_mut(event.client) {
            if event.hand == Hand::Main {
                if inv.slot(held_item.slot()).item == ItemKind::Bow {
                    draw_tick.0 = server.current_tick();
                }
            }
        }
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
struct ActionQuery {
    entity: Entity,
    held_item: &'static HeldItem,
    inv: &'static mut Inventory,
    pos: &'static Position,
    look: &'static Look,
    yaw: &'static HeadYaw,
    layer: &'static EntityLayerId,
    draw_tick: &'static mut BowDrawTick,
}
fn handle_player_actions(
    mut players: Query<ActionQuery>,
    mut clients: Query<&mut Client>,
    mut packets: EventReader<PacketEvent>,
    mut commands: Commands,
    server: Res<Server>,
) {
    for packet in packets.read() {
        if let Some(pkt) = packet.decode::<PlayerActionC2s>() {
            let Ok(mut player) = players.get_mut(packet.client) else {
                continue;
            };
            if pkt.action == PlayerAction::ReleaseUseItem
                && player.inv.slot(player.held_item.slot()).item == ItemKind::Bow
            {
                let Some(arrow_slot) = player.inv.first_slot_with_item(ItemKind::Arrow, 65) else {
                    continue;
                };

                let count = player.inv.slot(arrow_slot).count;
                player.inv.set_slot_amount(arrow_slot, count - 1);
                for mut client in clients.iter_mut() {
                    client.play_sound(
                        Sound::EntityArrowShoot,
                        SoundCategory::Player,
                        player.pos.0,
                        1.0,
                        1.0,
                    );
                }

                let rad_yaw = player.yaw.0.to_radians();
                let rad_pitch = player.look.pitch.to_radians();
                let hspeed = rad_pitch.cos();
                let x = -rad_yaw.sin() * hspeed;
                let y = -rad_pitch.sin();
                let z = rad_yaw.cos() * hspeed;

                let mag = (x * x + y * y + z * z).sqrt();

                let tick_diff = server.current_tick() - player.draw_tick.0;

                let vel = Vec3::new(
                    x / mag,
                    y / mag,
                    z / mag,
                ) * tick_diff.clamp(0, 20) as f32 * 3.0;
                let dir = vel.normalize().as_dvec3() * 0.5;
                let arrow_id = commands
                    .spawn(ArrowEntityBundle {
                        position: Position(DVec3::new(
                            player.pos.0.x + dir.x,
                            player.pos.0.y + 1.62,
                            player.pos.0.z + dir.z,
                        )),
                        look: *player.look,
                        head_yaw: *player.yaw,
                        velocity: Velocity(vel),
                        layer: *player.layer,
                        ..Default::default()
                    })
                    .id();
                commands
                    .entity(arrow_id)
                    .insert(ProjectileOwner(player.entity));

                player.draw_tick.0 = i64::MAX;
            }
        }
    }
}

pub fn apply_arrow_physics(
    mut arrows: Query<(Entity, &mut Position, &mut Velocity), With<ArrowEntity>>,
    players: Query<(Entity, &Position, &Velocity), (With<Client>, Without<ArrowEntity>)>,
    mut collisions: EventWriter<ProjectileCollisionEvent>,
    mut commands: Commands,
) {
    for (entity, mut pos, mut vel) in arrows.iter_mut() {
        pos.0 += DVec3::from(vel.0) / 20.0;

        // Gravity
        vel.0.y -= 1.0;

        // Air resistance
        vel.0 *= 0.99;

        // Check for collisions (Arrow's have a hitbox of 0.5x0.5x0.5 and players have a hitbox of 0.6x1.8x0.6)
        let arrow_shape = Cuboid::new(Vector::new(0.5, 0.5, 0.5));
        let arrow_iso = Isometry3::new(Vector::new(pos.0.x as f32, pos.0.y as f32, pos.0.z as f32), na::zero());
        let arrow_vel = Vector::new(vel.0.x / 100.0, vel.0.y / 100.0, vel.0.z / 100.0);

        let player_shape = Cuboid::new(Vector::new(0.6, 0.9, 0.6));
        
        for (player_entity, player_pos, player_vel) in players.iter() {
            let player_iso = Isometry3::new(Vector::new(player_pos.0.x as f32, player_pos.0.y as f32 + 0.9, player_pos.0.z as f32), na::zero());
            let player_vel = Vector::new(player_vel.0.x / 100.0, player_vel.0.y / 100.0, player_vel.0.z / 100.0);

            if let Some(_) = cast_shapes(&arrow_iso, &arrow_vel, &arrow_shape, &player_iso, &player_vel, &player_shape, ShapeCastOptions::with_max_time_of_impact(1.0)).unwrap() {
                commands.entity(entity).insert(Despawned);
                collisions.send(ProjectileCollisionEvent {
                    arrow: entity,
                    player: player_entity,
                });
            }
        }
    }
}

fn cleanup_arrows(
    arrows: Query<(Entity, &Position), With<ArrowEntity>>,
    mut commands: Commands,
) {
    for (entity, pos) in arrows.iter() {
        if pos.0.y < -50.0 {
            commands.entity(entity).insert(Despawned);
        }
    }
}