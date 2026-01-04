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

use std::marker::PhantomData;
use std::path::PathBuf;
use bevy_ecs::query::QueryData;
use minibit_lib::duels::*;
use minibit_lib::player::InteractionBroadcastPlugin;
use minibit_lib::projectiles::*;
use valence::entity::living::Health;
use valence::entity::Velocity;
use valence::entity::{EntityId, EntityStatuses};
use valence::equipment::EquipmentInventorySync;
use valence::math::Vec3Swizzles;
use valence::prelude::*;
use valence::protocol::packets::play::DamageTiltS2c;
use valence::protocol::sound::SoundCategory;
use valence::protocol::Sound;
use valence::protocol::VarInt;
use valence::protocol::WritePacket;

pub fn main(path: PathBuf) {
    App::new()
        .add_plugins(DuelsPlugin::<DefaultDuelsConfig> { path, default_gamemode: GameMode::Adventure, copy_map: false, phantom: PhantomData })
        .add_plugins(DefaultPlugins)
        .add_plugins((InteractionBroadcastPlugin, ProjectilePlugin))
        .add_systems(
            EventLoopUpdate,
            handle_combat_events,
        )
        .add_systems(
            Update,
            (
                init_clients,
                gamestage_change.after(minibit_lib::duels::gameloop::<DefaultDuelsConfig>),
                end_game.after(minibit_lib::duels::map::end_game::<DefaultDuelsConfig>),
                handle_collision_events,
                handle_oob_clients,
            ),
        )
        .run();
}

fn init_clients(clients: Query<Entity, Added<Client>>, mut commands: Commands) {
    for client in clients.iter() {
        commands.entity(client).insert(EquipmentInventorySync);
    }
}

fn gamestage_change(
    mut clients: Query<&mut Inventory, With<Client>>,
    games: Query<&Entities>,
    mut event: EventReader<GameStageEvent>,
) {
    for event in event.read() {
        if event.stage != 4 {
            continue;
        }
        if let Ok(entities) = games.get(event.game_id) {
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

#[derive(QueryData)]
#[query_data(mutable)]
struct CombatQuery {
    client: &'static mut Client,
    id: &'static EntityId,
    pos: &'static Position,
    old_pos: &'static OldPosition,
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

        damage_player(
            &mut attacker,
            &mut victim,
            1.0,
            Vec3::new(dir.x * knockback_xz, knockback_y, dir.y * knockback_xz),
            &mut end_game,
        );

        attacker.state.has_bonus_knockback = false;
    }
}

fn handle_collision_events(
    mut clients: Query<CombatQuery>,
    arrows: Query<(&Velocity, &ProjectileOwner)>,
    mut collisions: EventReader<ProjectileCollisionEvent>,
    mut end_game: EventWriter<EndGameEvent>,
) {
    for event in collisions.read() {
        if let Ok((vel, owner)) = arrows.get(event.arrow) {
            if let Ok([mut attacker, mut victim]) = clients.get_many_mut([owner.0, event.player]) {
                damage_player(
                    &mut attacker,
                    &mut victim,
                    0.13 * vel.0.length(),
                    Vec3::new(0.0, 0.0, 0.0),
                    &mut end_game,
                );
                attacker.client.play_sound(
                    Sound::EntityArrowHitPlayer,
                    SoundCategory::Player,
                    attacker.pos.0,
                    1.0,
                    1.0,
                );
            }
        }
    }
}

fn handle_oob_clients(
    positions: Query<(&mut Position, &PlayerGameState), With<Client>>,
    mut end_game: EventWriter<EndGameEvent>,
) {
    for (pos, gamestate) in positions.iter() {
        if pos.0.y < 0.0 {
            if gamestate.game_id.is_some() {
                end_game.send(EndGameEvent {
                    game_id: gamestate.game_id.unwrap(),
                    loser: gamestate.team,
                });
            }
        }
    }
}

// Helper functions below

fn damage_player(
    attacker: &mut CombatQueryItem,
    victim: &mut CombatQueryItem,
    damage: f32,
    velocity: Vec3,
    end_game: &mut EventWriter<EndGameEvent>,
) {
    let old_vel = Vec3::new(
        (victim.pos.0.x - victim.old_pos.get().x) as f32,
        (victim.pos.0.y - victim.old_pos.get().y) as f32,
        (victim.pos.0.z - victim.old_pos.get().z) as f32,
    );

    victim
        .client
        .set_velocity(old_vel + velocity);

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

    if victim.health.0 <= damage {
        end_game.send(EndGameEvent {
            game_id: victim.gamestate.game_id.unwrap(),
            loser: victim.gamestate.team,
        });
    } else {
        victim.health.0 -= damage;
    }
}