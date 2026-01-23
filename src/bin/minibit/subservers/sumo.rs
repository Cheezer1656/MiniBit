#![allow(clippy::type_complexity)]

use std::marker::PhantomData;
use bevy_ecs::query::QueryData;
use minibit_lib::duels::{CombatState, DefaultDuelsConfig, DuelsPlugin, PlayerGameState};
use valence::entity::{EntityId, EntityStatuses};
use valence::math::Vec3Swizzles;
use valence::prelude::*;
use valence::protocol::packets::play::DamageTiltS2c;
use valence::protocol::sound::SoundCategory;
use valence::protocol::{Sound, WritePacket};
use valence::protocol::VarInt;
use minibit_lib::duels::oob::{OobMode, OobPlugin};
use crate::ServerConfig;

pub fn main(config: ServerConfig) {
    App::new()
        .add_plugins(DuelsPlugin::<DefaultDuelsConfig> {
            path: config.path,
            network_config: config.network,
            default_gamemode: GameMode::Adventure,
            copy_map: false,
            phantom: PhantomData
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(OobPlugin {
            mode: OobMode::GameEndEvent,
            bounds_y: 0.0..,
        })
        .add_systems(EventLoopUpdate, handle_combat_events)
        .run();
}

#[derive(QueryData)]
#[query_data(mutable)]
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
        interact: interaction,
        ..
    } in interact_entity.read()
    {
        let Ok([mut attacker, mut victim]) = clients.get_many_mut([attacker_client, victim_client])
        else {
            continue;
        };

        if interaction != EntityInteraction::Attack
            || server.current_tick() - victim.state.last_attacked_tick < 10
            || attacker.gamestate.game_id != victim.gamestate.game_id
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
        victim.client.write_packet(&DamageTiltS2c {
            entity_id: VarInt(0),
            yaw: 0.0,
        });
        attacker.client.play_sound(
            Sound::EntityPlayerHurt,
            SoundCategory::Player,
            victim.pos.0,
            1.0,
            1.0,
        );
        attacker.client.write_packet(&DamageTiltS2c {
            entity_id: VarInt(victim.id.get()),
            yaw: 0.0,
        });
    }
}
