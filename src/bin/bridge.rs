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

#[path = "../lib/mod.rs"]
mod lib;

use bevy_ecs::query::QueryData;
use lib::duels::*;
use lib::player::*;
use lib::projectiles::*;
use lib::world::*;
use valence::entity::living::Absorption;
use valence::entity::living::Health;
use valence::entity::{EntityId, EntityStatuses};
use valence::event_loop::PacketEvent;
use valence::interact_item::InteractItemEvent;
use valence::inventory::HeldItem;
use valence::inventory::PlayerAction;
use valence::math::Vec3Swizzles;
use valence::nbt::compound;
use valence::prelude::*;
use valence::protocol::packets::play::DamageTiltS2c;
use valence::protocol::packets::play::PlayerActionC2s;
use valence::protocol::sound::SoundCategory;
use valence::protocol::Sound;
use valence::protocol::VarInt;
use valence::protocol::WritePacket;

#[derive(Event)]
struct DeathEvent(Entity, bool);

#[derive(Event)]
struct ScoreEvent {
    game: Entity,
    team: u8,
}

#[derive(Event)]
struct MessageEvent {
    game: Entity,
    msg: Text,
}

#[derive(Component)]
struct EatingStartTick(pub i64);

fn main() {
    App::new()
        .add_plugins(DuelsPlugin {
            default_gamemode: GameMode::Survival,
        })
        .add_plugins(DefaultPlugins)
        .add_plugins((
            InvBroadcastPlugin,
            ProjectilePlugin,
            DiggingPlugin {
                whitelist: vec![
                    BlockKind::BlueTerracotta,
                    BlockKind::RedTerracotta,
                    BlockKind::WhiteTerracotta,
                ],
            },
            PlacingPlugin { build_limit: 100 },
        ))
        .add_event::<DeathEvent>()
        .add_event::<ScoreEvent>()
        .add_event::<MessageEvent>()
        .add_systems(EventLoopUpdate, handle_combat_events)
        .add_systems(
            Update,
            (
                init_clients,
                start_game.after(lib::duels::start_game),
                gamestage_change,
                end_game,
                check_goals,
                set_use_tick,
                eat_gapple,
                cancel_gapple,
                handle_collision_events,
                handle_death,
                handle_score.after(check_goals).before(handle_death),
                handle_oob_clients,
                game_broadcast,
            ),
        )
        .run();
}

fn init_clients(clients: Query<Entity, Added<Client>>, mut commands: Commands) {
    for entity in clients.iter() {
        commands.entity(entity).insert(EatingStartTick(i64::MAX));
    }
}

fn start_game(
    mut clients: Query<(&mut Inventory, &PlayerGameState), With<Client>>,
    mut games: Query<(&Entities, &mut GameData)>,
    mut start_game: EventReader<StartGameEvent>,
) {
    for event in start_game.read() {
        if let Ok((entities, mut data)) = games.get_mut(event.0) {
            data.0.insert(0, DataValue::Int(0));
            data.0.insert(1, DataValue::Int(0));

            for entity in entities.0.iter() {
                if let Ok((mut inventory, gamestate)) = clients.get_mut(*entity) {
                    fill_inventory(&mut inventory, gamestate.team);
                }
            }
        }
    }
}

fn fill_inventory(inv: &mut Inventory, team: u8) {
    let armor_nbt = Some(compound! {
        "display" => compound! {
            "color" => match team {
                0 => 3949738,
                1 => 11546150,
                _ => 0,
            }
        }
    });
    let block_type = match team {
        0 => ItemKind::BlueTerracotta,
        1 => ItemKind::RedTerracotta,
        _ => ItemKind::WhiteTerracotta,
    };
    inv.set_slot(
        6,
        ItemStack::new(ItemKind::LeatherChestplate, 1, armor_nbt.clone()),
    );
    inv.set_slot(
        7,
        ItemStack::new(ItemKind::LeatherLeggings, 1, armor_nbt.clone()),
    );
    inv.set_slot(8, ItemStack::new(ItemKind::LeatherBoots, 1, armor_nbt));
    inv.set_slot(36, ItemStack::new(ItemKind::IronSword, 1, None));
    inv.set_slot(37, ItemStack::new(ItemKind::Bow, 1, None));
    inv.set_slot(38, ItemStack::new(ItemKind::DiamondPickaxe, 1, None));
    inv.set_slot(39, ItemStack::new(block_type, 64, None));
    inv.set_slot(40, ItemStack::new(block_type, 64, None));
    inv.set_slot(41, ItemStack::new(ItemKind::GoldenApple, 8, None));
    inv.set_slot(44, ItemStack::new(ItemKind::Arrow, 10, None));
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

fn check_goals(
    clients: Query<(Entity, &Position, &PlayerGameState), With<Client>>,
    config: Res<DuelsConfig>,
    mut scores: EventWriter<ScoreEvent>,
    mut deaths: EventWriter<DeathEvent>,
) {
    for (entity, pos, gamestate) in clients.iter() {
        if let Some(game_id) = gamestate.game_id {
            if let Some(data) = &config.other {
                let x = pos.0.x.floor() as isize;
                let y = pos.0.y.floor() as isize;
                let z = pos.0.z.floor() as isize;
                if data[0] <= x && data[1] >= x && y == data[2] && data[3] <= z && data[4] >= z {
                    match gamestate.team {
                        1 => {
                            scores.send(ScoreEvent {
                                game: game_id,
                                team: 1,
                            });
                        }
                        _ => {
                            deaths.send(DeathEvent(entity, true));
                        }
                    }
                } else if data[5] <= x
                    && data[6] >= x
                    && y == data[7]
                    && data[8] <= z
                    && data[9] >= z
                {
                    match gamestate.team {
                        0 => {
                            scores.send(ScoreEvent {
                                game: game_id,
                                team: 0,
                            });
                        }
                        _ => {
                            deaths.send(DeathEvent(entity, true));
                        }
                    }
                }
            }
        }
    }
}

fn set_use_tick(
    mut clients: Query<(&Inventory, &HeldItem, &mut EatingStartTick), With<Client>>,
    mut events: EventReader<InteractItemEvent>,
    server: Res<Server>,
) {
    for event in events.read() {
        if let Ok((inv, held_item, mut eat_tick)) = clients.get_mut(event.client) {
            if event.hand == Hand::Main {
                if inv.slot(held_item.slot()).item == ItemKind::GoldenApple {
                    eat_tick.0 = server.current_tick();
                }
            }
        }
    }
}

fn eat_gapple(
    mut clients: Query<
        (
            &mut Client,
            &mut Health,
            &mut Absorption,
            &mut Inventory,
            &HeldItem,
            &mut EatingStartTick,
        ),
        With<Client>,
    >,
    server: Res<Server>,
) {
    for (mut client, mut health, mut absorption, mut inv, held_item, mut eat_tick) in
        clients.iter_mut()
    {
        let slot = held_item.slot();
        if inv.slot(slot).item != ItemKind::GoldenApple {
            eat_tick.0 = i64::MAX;
            continue;
        }
        if server.current_tick() - eat_tick.0 > 32 {
            eat_tick.0 = i64::MAX;
            client.trigger_status(EntityStatus::ConsumeItem);
            health.0 = 20.0;
            absorption.0 = 4.0;
            let count = inv.slot(slot).count;
            inv.set_slot_amount(held_item.slot(), count - 1);
        }
    }
}

fn cancel_gapple(
    mut clients: Query<(&Inventory, &HeldItem, &mut EatingStartTick), With<Client>>,
    mut packets: EventReader<PacketEvent>,
) {
    for packet in packets.read() {
        if let Some(pkt) = packet.decode::<PlayerActionC2s>() {
            if pkt.action == PlayerAction::ReleaseUseItem {
                if let Ok((inv, held_item, mut eat_tick)) = clients.get_mut(packet.client) {
                    if inv.slot(held_item.slot()).item == ItemKind::GoldenApple {
                        eat_tick.0 = i64::MAX;
                    }
                }
            }
        }
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
struct CombatQuery {
    entity: Entity,
    client: &'static mut Client,
    id: &'static EntityId,
    pos: &'static Position,
    old_pos: &'static OldPosition,
    state: &'static mut CombatState,
    statuses: &'static mut EntityStatuses,
    gamestate: &'static PlayerGameState,
    health: &'static mut Health,
    absorption: &'static mut Absorption,
    held_item: &'static HeldItem,
    inv: &'static Inventory,
}

fn handle_combat_events(
    server: Res<Server>,
    mut clients: Query<CombatQuery>,
    mut sprinting: EventReader<SprintEvent>,
    mut interact_entity: EventReader<InteractEntityEvent>,
    mut deaths: EventWriter<DeathEvent>,
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

        let dmg = match attacker.inv.slot(attacker.held_item.slot()).item {
            ItemKind::IronSword => 6.0,
            ItemKind::DiamondPickaxe => 5.0,
            _ => 1.0,
        };

        damage_player(
            &mut attacker,
            &mut victim,
            dmg,
            Vec3::new(dir.x * knockback_xz, knockback_y, dir.y * knockback_xz),
            &mut deaths,
        );

        attacker.state.has_bonus_knockback = false;
    }
}

fn handle_collision_events(
    mut clients: Query<CombatQuery>,
    arrows: Query<&ProjectileOwner>,
    mut collisions: EventReader<ProjectileCollisionEvent>,
    mut deaths: EventWriter<DeathEvent>,
) {
    for event in collisions.read() {
        if let Ok(owner) = arrows.get(event.arrow) {
            if let Ok([mut attacker, mut victim]) = clients.get_many_mut([owner.0, event.player]) {
                damage_player(
                    &mut attacker,
                    &mut victim,
                    6.0,
                    Vec3::new(0.0, 0.0, 0.0),
                    &mut deaths,
                );
            }
        }
    }
}

fn handle_oob_clients(
    positions: Query<(Entity, &Position, &PlayerGameState), With<Client>>,
    mut deaths: EventWriter<DeathEvent>,
) {
    for (entity, pos, gamestate) in positions.iter() {
        if pos.0.y < 0.0 {
            if gamestate.game_id.is_some() {
                deaths.send(DeathEvent(entity, true));
            }
        }
    }
}

fn handle_death(
    mut clients: Query<
        (
            &mut Position,
            &mut Look,
            &mut HeadYaw,
            &mut Health,
            &mut Absorption,
            &mut Inventory,
            &Username,
            &PlayerGameState,
        ),
        With<Client>,
    >,
    games: Query<&MapIndex>,
    mut deaths: EventReader<DeathEvent>,
    mut broadcasts: EventWriter<MessageEvent>,
    config: Res<DuelsConfig>,
) {
    for DeathEvent(entity, show) in deaths.read() {
        if let Ok((
            mut pos,
            mut look,
            mut head_yaw,
            mut health,
            mut absorption,
            mut inventory,
            username,
            gamestate,
        )) = clients.get_mut(*entity)
        {
            if let Some(game_id) = gamestate.game_id {
                if let Ok(map_index) = games.get(game_id) {
                    let spawn = &config.worlds[map_index.0].spawns[gamestate.team as usize];
                    pos.0 = spawn.pos.into();
                    look.yaw = spawn.rot[0];
                    look.pitch = spawn.rot[1];
                    head_yaw.0 = spawn.rot[0];
                    health.0 = 20.0;
                    absorption.0 = 0.0;
                    for slot in 0..inventory.slot_count() {
                        inventory.set_slot(slot, ItemStack::EMPTY);
                    }
                    fill_inventory(&mut inventory, gamestate.team);
                    if *show {
                        broadcasts.send(MessageEvent {
                            game: game_id,
                            msg: Text::from(username.0.clone()).color(if gamestate.team == 0 {
                                Color::BLUE
                            } else {
                                Color::RED
                            }) + Text::from(" has died!").color(Color::GRAY),
                        });
                    }
                }
            }
        }
    }
}

fn handle_score(
    mut games: Query<(&Entities, &mut GameData)>,
    mut scores: EventReader<ScoreEvent>,
    mut deaths: EventWriter<DeathEvent>,
    mut broadcasts: EventWriter<MessageEvent>,
    mut end_game: EventWriter<EndGameEvent>,
) {
    for ScoreEvent { game, team } in scores.read() {
        if let Ok((entities, mut data)) = games.get_mut(*game) {
            let mut score = 0;
            if let Some(DataValue::Int(old_score)) = data.0.get(&(*team as usize)) {
                score = *old_score + 1;
            }
            data.0.insert(*team as usize, DataValue::Int(score));
            for entity in entities.0.iter() {
                deaths.send(DeathEvent(*entity, false));
            }
            broadcasts.send(MessageEvent {
                game: *game,
                msg: if *team == 0 {
                    Text::from("Team Blue").color(Color::BLUE)
                } else {
                    Text::from("Team Red").color(Color::RED)
                } + Text::from(" scored! (").color(Color::GRAY)
                    + Text::from(score.to_string()).color(Color::GOLD)
                    + Text::from("/5)").color(Color::GRAY),
            });
            if score >= 5 {
                end_game.send(EndGameEvent {
                    game_id: *game,
                    loser: if *team == 0 { 1 } else { 0 },
                });
            }
        }
    }
}

fn game_broadcast(
    mut clients: Query<&mut Client>,
    games: Query<&Entities>,
    mut broadcasts: EventReader<MessageEvent>,
) {
    for MessageEvent { game, msg } in broadcasts.read() {
        if let Ok(entities) = games.get(*game) {
            for entity in entities.0.iter() {
                if let Ok(mut client) = clients.get_mut(*entity) {
                    client.send_chat_message(msg);
                }
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
    deaths: &mut EventWriter<DeathEvent>,
) {
    let old_vel = Vec3::new(
        (victim.pos.0.x - victim.old_pos.get().x) as f32,
        (victim.pos.0.y - victim.old_pos.get().y) as f32,
        (victim.pos.0.z - victim.old_pos.get().z) as f32,
    );

    victim.client.set_velocity(old_vel + velocity);

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

    let mut new_damage = damage;
    if victim.absorption.0 > 0.0 {
        new_damage -= damage.min(victim.absorption.0);
        victim.absorption.0 -= damage.min(victim.absorption.0);
    }
    if victim.health.0 <= new_damage {
        deaths.send(DeathEvent(victim.entity, true));
    } else {
        victim.health.0 -= new_damage;
    }
}
