use parry3d::{math::Vector, na::{self, Isometry3}, query::{cast_shapes, ShapeCastOptions}, shape::Cuboid};
use valence::{entity::{arrow::ArrowEntity, OnGround, Velocity}, prelude::*};

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
            .add_systems(Update, apply_arrow_physics);
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