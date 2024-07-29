#![allow(dead_code)]

use bevy_ecs::query::WorldQuery;
use parry3d::{math::Vector, na::{self, Isometry3}, query::{cast_shapes, ShapeCastOptions}, shape::Cuboid};
use valence::{entity::{arrow::{ArrowEntity, ArrowEntityBundle}, OnGround, Velocity}, event_loop::PacketEvent, inventory::PlayerAction, prelude::*, protocol::{packets::play::PlayerActionC2s, sound::SoundCategory, Sound}};

use super::duels::CombatState;

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
            .add_systems(EventLoopUpdate, handle_player_actions)
            .add_systems(Update, apply_arrow_physics);
    }
}

#[derive(WorldQuery)]
#[world_query(mutable)]
struct ActionQuery {
    entity: Entity,
    inv: &'static mut Inventory,
    pos: &'static Position,
    look: &'static Look,
    yaw: &'static HeadYaw,
    layer: &'static EntityLayerId,
    state: &'static mut CombatState,
}
fn handle_player_actions(
    mut players: Query<ActionQuery>,
    mut clients: Query<&mut Client>,
    mut packets: EventReader<PacketEvent>,
    mut commands: Commands,
) {
    for packet in packets.read() {
        if let Some(pkt) = packet.decode::<PlayerActionC2s>() {
            let Ok(mut player) = players.get_mut(packet.client) else {
                continue;
            };
            if pkt.action == PlayerAction::ReleaseUseItem
                && player.inv.slot(36).item == ItemKind::Bow
                && player.inv.slot(44).item == ItemKind::Arrow
            {
                let count = player.inv.slot(44).count;
                player.inv.set_slot_amount(44, count - 1);
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
                let vel = Vec3::new(
                    -rad_yaw.sin() * hspeed,
                    -rad_pitch.sin(),
                    rad_yaw.cos() * hspeed,
                ) * 30.0;
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
            }
        }
    }
}

pub fn apply_arrow_physics(
    mut arrows: Query<(Entity, &mut Position, &mut Velocity, &mut OnGround), With<ArrowEntity>>,
    players: Query<(Entity, &Position, &Velocity), (With<Client>, Without<ArrowEntity>)>,
    mut collisions: EventWriter<ProjectileCollisionEvent>,
) {
    for (entity, mut pos, mut vel, mut on_ground) in arrows.iter_mut() {
        if on_ground.0 {
            continue;
        }

        pos.0 += DVec3::from(vel.0) / 20.0;

        // Gravity
        vel.0.y -= 20.0 / 20.0;

        // Air resistance
        vel.0 *= 1.0 - (0.99 / 20.0);

        // Check for collisions (Arrow's have a hitbox of 0.5x0.5x0.5 and players have a hitbox of 0.6x1.8x0.6)
        let arrow_shape = Cuboid::new(Vector::new(0.5, 0.5, 0.5));
        let arrow_iso = Isometry3::new(Vector::new(pos.0.x as f32, pos.0.y as f32, pos.0.z as f32), na::zero());
        let arrow_vel = Vector::new(vel.0.x / 100.0, vel.0.y / 100.0, vel.0.z / 100.0);

        let player_shape = Cuboid::new(Vector::new(0.6, 0.9, 0.6));
        
        for (player_entity, player_pos, player_vel) in players.iter() {
            let player_iso = Isometry3::new(Vector::new(player_pos.0.x as f32, player_pos.0.y as f32, player_pos.0.z as f32), na::zero());
            let player_vel = Vector::new(player_vel.0.x / 100.0, player_vel.0.y / 100.0, player_vel.0.z / 100.0);

            if let Some(_) = cast_shapes(&arrow_iso, &arrow_vel, &arrow_shape, &player_iso, &player_vel, &player_shape, ShapeCastOptions::with_max_time_of_impact(1.0)).unwrap() {
                on_ground.0 = true;
                collisions.send(ProjectileCollisionEvent {
                    arrow: entity,
                    player: player_entity,
                });
            }
        }
    }
}