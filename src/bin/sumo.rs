#![allow(clippy::type_complexity)]

#[path = "../libs/config.rs"]
mod config;
#[path = "../libs/game.rs"]
mod game;

use bevy_ecs::query::WorldQuery;
use config::*;
use game::*;
use valence::entity::{EntityId, EntityStatuses};
use valence::math::Vec3Swizzles;
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
        .add_systems(EventLoopUpdate, handle_combat_events)
        .add_systems(
            Update,
            (
                init_clients,
                despawn_disconnected_clients,
                handle_oob_clients,
                start_game.after(init_clients),
                end_game.after(handle_oob_clients),
                gameloop.after(start_game),
                chat_message,
            ),
        )
        .add_systems(PostUpdate, (handle_disconnect, check_queue))
        .run();
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
                    loser: gamestate.team
                });
            }
        }
    }
}