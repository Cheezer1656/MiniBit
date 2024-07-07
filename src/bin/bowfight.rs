#![allow(clippy::type_complexity)]

#[path = "../lib/mod.rs"]
mod lib;

use bevy_ecs::query::WorldQuery;
use lib::config::*;
use lib::game::*;
use valence::entity::arrow::ArrowEntity;
use valence::entity::arrow::ArrowEntityBundle;
use valence::entity::living::Health;
use valence::entity::Velocity;
use valence::entity::{EntityId, EntityStatuses};
use valence::event_loop::PacketEvent;
use valence::inventory::PlayerAction;
use valence::math::Vec3Swizzles;
use valence::protocol::packets::play::PlayerActionC2s;
use valence::protocol::sound::SoundCategory;
use valence::protocol::Sound;
use valence::{prelude::*, CompressionThreshold, ServerSettings};

pub fn main() {
    let config = match load_config() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("{}", e);
            return;
        }
    };

    App::new()
        .insert_resource(config.0)
        .insert_resource(ServerSettings {
            compression_threshold: CompressionThreshold(-1),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .insert_resource(config.1)
        .add_event::<StartGameEvent>()
        .add_event::<EndGameEvent>()
        .add_systems(Startup, setup)
        .add_systems(
            EventLoopUpdate,
            (handle_combat_events, handle_player_action),
        )
        .add_systems(
            Update,
            (
                init_clients,
                despawn_disconnected_clients,
                handle_oob_clients,
                lib::game::start_game.after(init_clients),
                start_game.after(lib::game::start_game),
                lib::game::end_game.after(handle_oob_clients),
                end_game.after(lib::game::end_game),
                gameloop.after(start_game),
                chat_message,
                handle_arrow_physics,
            ),
        )
        .add_systems(PostUpdate, (handle_disconnect, check_queue))
        .run();
}

fn start_game(
    mut clients: Query<&mut Inventory, With<Client>>,
    games: Query<&Entities>,
    mut start_game: EventReader<StartGameEvent>,
) {
    for event in start_game.read() {
        if let Ok(entities) = games.get(event.0) {
            for entity in entities.0.iter() {
                if let Ok(mut inventory) = clients.get_mut(*entity) {
                    inventory.set_slot(36, ItemStack::new(ItemKind::Bow, 1, None));
                    inventory.set_slot(44, ItemStack::new(ItemKind::Arrow, 10, None));
                }
            }
        }
    }
}

fn end_game(
    mut clients: Query<&mut Inventory, With<Client>>,
    games: Query<&Entities>,
    mut end_game: EventReader<EndGameEvent>,
) {
    for event in end_game.read() {
        if let Ok(entities) = games.get(event.game_id) {
            for entity in entities.0.iter() {
                if let Ok(mut inv) = clients.get_mut(*entity) {
                    for slot in 0..inv.slot_count() {
                        inv.set_slot(slot, ItemStack::EMPTY);
                    }
                }
            }
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
    health: &'static mut Health,
}

fn handle_combat_events(
    server: Res<Server>,
    mut clients: Query<CombatQuery>,
    mut sprinting: EventReader<SprintEvent>,
    mut interact_entity: EventReader<InteractEntityEvent>,
    mut end_game: EventWriter<EndGameEvent>,
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

        if attacker.gamestate.game_id != victim.gamestate.game_id
            || server.current_tick() - victim.state.last_attacked_tick < 10
        {
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

        if victim.health.0 <= 1.0 {
            end_game.send(EndGameEvent {
                game_id: victim.gamestate.game_id.unwrap(),
                loser: victim.gamestate.team,
            });
        } else {
            victim.health.0 -= 1.0;
        }
    }
}

#[derive(WorldQuery)]
#[world_query(mutable)]
struct ActionQuery {
    client: &'static mut Client,
    inv: &'static mut Inventory,
    pos: &'static Position,
    look: &'static Look,
    yaw: &'static HeadYaw,
    layer: &'static EntityLayerId,
    state: &'static mut CombatState,
}
fn handle_player_action(
    mut clients: Query<ActionQuery>,
    mut packets: EventReader<PacketEvent>,
    mut commands: Commands,
) {
    for packet in packets.read() {
        if let Some(pkt) = packet.decode::<PlayerActionC2s>() {
            let Ok(mut client) = clients.get_mut(packet.client) else {
                continue;
            };
            if pkt.action == PlayerAction::ReleaseUseItem
                && client.inv.slot(36).item == ItemKind::Bow
                && client.inv.slot(44).item == ItemKind::Arrow
            {
                let count = client.inv.slot(44).count;
                client.inv.set_slot_amount(44, count - 1);
                client.client.play_sound(
                    Sound::EntityArrowShoot,
                    SoundCategory::Player,
                    client.pos.0,
                    1.0,
                    1.0,
                );
                let rad_yaw = client.yaw.0.to_radians();
                let rad_pitch = client.look.pitch.to_radians();
                let hspeed = rad_pitch.cos();
                let vel = Vec3::new(
                    -rad_yaw.sin() * hspeed,
                    -rad_pitch.sin(),
                    rad_yaw.cos() * hspeed,
                ) * 30.0;
                let dir = vel.normalize().as_dvec3() * 0.5;
                println!("Vel: {:?}, Dir: {:?}", vel, dir);
                commands.spawn(ArrowEntityBundle {
                    position: Position(DVec3::new(
                        client.pos.0.x + dir.x,
                        client.pos.0.y + 1.62,
                        client.pos.0.z + dir.z,
                    )),
                    look: *client.look,
                    head_yaw: *client.yaw,
                    velocity: Velocity(vel),
                    layer: *client.layer,
                    ..Default::default()
                });
            }
        }
    }
}

fn handle_arrow_physics(
    mut arrows: Query<(&mut Position, &mut Velocity), With<ArrowEntity>>,
    mut clients: Query<
        (&PlayerGameState, &Position, &mut Health),
        (With<Client>, Without<ArrowEntity>),
    >,
    mut endgame: EventWriter<EndGameEvent>,
) {
    for (mut pos, mut velocity) in arrows.iter_mut() {
        pos.0 += DVec3::from(velocity.0) / 20.0;

        //add gravity
        velocity.0.y -= 20.0 / 20.0;

        //air friction
        velocity.0 *= 1.0 - (0.99 / 20.0);
        for (gamestate, player_pos, mut health) in clients.iter_mut() {
            if (pos.0.x - player_pos.0.x).abs() < 0.3
                && (pos.0.z - player_pos.0.z).abs() < 0.3
                && (pos.0.y - player_pos.0.y) < 1.8
                && (pos.0.y - player_pos.0.y) > 0.0
            {
                if health.0 <= 1.0 {
                    endgame.send(EndGameEvent {
                        game_id: gamestate.game_id.unwrap(),
                        loser: gamestate.team,
                    });
                } else {
                    health.0 -= 1.0;
                }
                
            }
        }
    }
}

fn handle_oob_clients(
    mut positions: Query<(&mut Position, &PlayerGameState), With<Client>>,
    mut end_game: EventWriter<EndGameEvent>,
    config: Res<ServerConfig>,
) {
    for (mut pos, gamestate) in positions.iter_mut() {
        if pos.0.y < 0.0 {
            pos.set(config.spawn_pos);
            if gamestate.game_id.is_some() {
                end_game.send(EndGameEvent {
                    game_id: gamestate.game_id.unwrap(),
                    loser: gamestate.team,
                });
            }
        }
    }
}
