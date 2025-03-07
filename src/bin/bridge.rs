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
use std::time::SystemTime;

use bevy_ecs::query::QueryData;
use minibit_lib::color::ArmorColors;
use minibit_lib::config::WorldValue;
use minibit_lib::damage::calc_dmg;
use minibit_lib::damage::calc_dmg_with_weapon;
use minibit_lib::duels::*;
use minibit_lib::player::*;
use minibit_lib::projectiles::*;
use minibit_lib::world::*;
use serde::Deserialize;
use valence::entity::living::Absorption;
use valence::entity::living::Health;
use valence::entity::Velocity;
use valence::entity::{EntityId, EntityStatuses};
use valence::equipment::EquipmentInventorySync;
use valence::event_loop::PacketEvent;
use valence::interact_item::InteractItemEvent;
use valence::inventory::HeldItem;
use valence::inventory::PlayerAction;
use valence::math::IVec3;
use valence::math::Vec3Swizzles;
use valence::nbt::compound;
use valence::nbt::List;
use valence::prelude::*;
use valence::protocol::packets::play::DamageTiltS2c;
use valence::protocol::packets::play::ExperienceBarUpdateS2c;
use valence::protocol::packets::play::PlayerActionC2s;
use valence::protocol::sound::SoundCategory;
use valence::protocol::Sound;
use valence::protocol::VarInt;
use valence::protocol::WritePacket;

#[derive(Event)]
struct DeathEvent(Entity, bool);

#[derive(Event)]
struct ScoreEvent (Entity);

#[derive(Event)]
struct MessageEvent {
    game: Entity,
    msg: Text,
}

#[derive(Component)]
struct BowStatus {
    cooldown: i64,
    slot: u16,
}

impl Default for BowStatus {
    fn default() -> Self {
        Self {
            cooldown: i64::MAX,
            slot: 44,
        }
    }
}

#[derive(Resource, Deserialize)]
struct BridgeConfig {
    worlds: Vec<WorldValue>,
    goals: Vec<[i32; 5]>,
    block_restrictions: Vec<[i32; 6]>,
}

impl DuelsConfig for BridgeConfig {
    fn worlds(&self) -> &Vec<WorldValue> {
        &self.worlds
    }
}

#[derive(Component)]
struct EatingStartTick(pub i64);

fn main() {
    App::new()
        .add_plugins(DuelsPlugin::<BridgeConfig> {
            default_gamemode: GameMode::Survival,
            copy_map: true,
            phantom: PhantomData,
        })
        .add_plugins(DefaultPlugins)
        .add_plugins((
            InteractionBroadcastPlugin,
            DisableDropPlugin,
            ProjectilePlugin,
            DiggingPlugin {
                whitelist: vec![
                    BlockKind::BlueTerracotta,
                    BlockKind::RedTerracotta,
                    BlockKind::WhiteTerracotta,
                ],
            },
            PlacingPlugin {
                max_x: 16,
                min_x: -16,
                max_y: 100,
                min_y: 19,
                max_z: 60,
                min_z: -60,
            },
        ))
        .add_event::<DeathEvent>()
        .add_event::<ScoreEvent>()
        .add_event::<MessageEvent>()
        .add_systems(Startup, setup)
        .add_systems(EventLoopUpdate, handle_combat_events)
        .add_systems(
            Update,
            (
                init_clients,
                start_game,
                gamestage_change,
                end_game,
                check_goals,
                update_bow_cooldown,
                set_use_tick,
                eat_gapple,
                cancel_gapple,
                handle_collision_events,
                handle_death,
                play_death_sound.before(handle_death),
                handle_score.after(check_goals).before(handle_death),
                handle_oob_clients,
                game_broadcast,
            ),
        )
        .run();
}

fn setup(mut commands: Commands, server_config: Res<BridgeConfig>) {
        let areas = server_config.block_restrictions.iter().map(|area| BlockArea {
            min: IVec3::new(area[0], area[1], area[2]),
            max: IVec3::new(area[3], area[4], area[5]),
        }).collect();
        commands.insert_resource(PlacingRestrictions { areas });
}

fn init_clients(clients: Query<Entity, Added<Client>>, mut commands: Commands) {
    for entity in clients.iter() {
        commands.entity(entity).insert((EquipmentInventorySync, EatingStartTick(i64::MAX), BowStatus::default()));
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

fn gamestage_change(
    mut layers: Query<&mut ChunkLayer>,
    games: Query<(&MapIndex, &EntityLayerId)>,
    server_config: Res<BridgeConfig>,
    mut gamestage: EventReader<GameStageEvent>,
) {
    for event in gamestage.read() {
        let Ok((map_idx, layer_id)) = games.get(event.game_id) else {
            continue;
        };
        let Ok(mut layer) = layers.get_mut(layer_id.0) else {
            continue;
        };

        match event.stage {
            0 | 2 => { // For some reason the blocks are overwritten at the start of the game, so we need to reapply them
                for i in 0..2 {
                    let spawn_pos =
                        DVec3::from_array(server_config.worlds()[map_idx.0].spawns[i].pos);
                    // Create the cage
                    for x in -2..=2 {
                        for y in -1..=3 {
                            for z in -2..=2 {
                                if x < -1 || x > 1 || y < 0 || y > 2 || z < -1 || z > 1 {
                                    layer.set_block(
                                        spawn_pos + DVec3::new(x as f64, y as f64, z as f64),
                                        BlockState::GLASS,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            4 => {
                // Clear the cages
                for i in 0..2 {
                    let spawn_pos =
                        DVec3::from_array(server_config.worlds()[map_idx.0].spawns[i].pos);
                    for x in -2..=2 {
                        for y in -1..=3 {
                            for z in -2..=2 {
                                layer.set_block(
                                    spawn_pos + DVec3::new(x as f64, y as f64, z as f64),
                                    BlockState::AIR,
                                );
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn fill_inventory(inv: &mut Inventory, team: u8) {
    let armor_nbt = Some(compound! {
        "display" => compound! {
            "color" => match team {
                0 => ArmorColors::Blue as i32,
                1 => ArmorColors::Red as i32,
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
    inv.set_slot(38, ItemStack::new(ItemKind::DiamondPickaxe, 1, Some(compound! {
        "Enchantments" => List::Compound(vec! [
            compound! {
                "id" => "efficiency",
                "lvl" => 2
            }
        ])
    })));
    inv.set_slot(39, ItemStack::new(block_type, 64, None));
    inv.set_slot(40, ItemStack::new(block_type, 64, None));
    inv.set_slot(41, ItemStack::new(ItemKind::GoldenApple, 8, None));
    inv.set_slot(44, ItemStack::new(ItemKind::Arrow, 1, None));
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
    config: Res<BridgeConfig>,
    mut scores: EventWriter<ScoreEvent>,
    mut deaths: EventWriter<DeathEvent>,
) {
    for (entity, pos, gamestate) in clients.iter() {
        if let Some(_) = gamestate.game_id {
            for (i, goal) in config.goals.iter().enumerate() {
                if goal[0] <= pos.0.x as i32
                    && goal[1] >= pos.0.x as i32
                    && goal[2] == pos.0.y as i32
                    && goal[3] <= pos.0.z as i32
                    && goal[4] >= pos.0.z as i32
                {
                    if gamestate.team == i as u8 {
                        deaths.send(DeathEvent(entity, true));
                    } else {
                        scores.send(ScoreEvent(entity));
                    }
                }
            }
        }
    }
}

fn update_bow_cooldown(
    mut clients: Query<(&mut Client, &mut Inventory, &CursorItem, &mut BowStatus)>,
    server: Res<Server>,
) {
    for (mut client, mut inv, cursor_item, mut bow_status) in clients.iter_mut() {
        let tick = server.current_tick();
        if let Some(slot) = inv.first_slot_with_item(ItemKind::Arrow, 2) {
            if inv.slot(slot).count > 0 || cursor_item.0.item == ItemKind::Arrow {
                bow_status.cooldown = tick + 60;
                bow_status.slot = slot;
                continue;
            }
        }
        let tick_diff = bow_status.cooldown - tick;
        if tick_diff % 5 == 0 {
            client.write_packet(&ExperienceBarUpdateS2c {
                bar: tick_diff as f32 / 60.0,
                level: VarInt(0),
                total_xp: VarInt(0),
            });
        } else if tick_diff == 59 {
            client.write_packet(&ExperienceBarUpdateS2c {
                bar: 1.0,
                level: VarInt(0),
                total_xp: VarInt(0),
            });
        }
        if bow_status.cooldown < tick {
            inv.set_slot(bow_status.slot, ItemStack::new(ItemKind::Arrow, 1, None));
            client.write_packet(&ExperienceBarUpdateS2c {
                bar: 0.0,
                level: VarInt(0),
                total_xp: VarInt(0),
            });
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
            || attacker.gamestate.team == victim.gamestate.team
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

        let dmg = calc_dmg_with_weapon(
            attacker.inv.slot(attacker.held_item.slot()).item,
            victim.inv.slot(5).item,
            victim.inv.slot(6).item,
            victim.inv.slot(7).item,
            victim.inv.slot(8).item
        );

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
    arrows: Query<(&Velocity, &ProjectileOwner)>,
    mut collisions: EventReader<ProjectileCollisionEvent>,
    mut deaths: EventWriter<DeathEvent>,
) {
    for event in collisions.read() {
        if let Ok((vel, owner)) = arrows.get(event.arrow) {
            if let Ok([mut attacker, mut victim]) = clients.get_many_mut([owner.0, event.player]) {
                if attacker.gamestate.team == victim.gamestate.team {
                    continue;
                }

                // TODO: Make the damage accurate
                let dmg = calc_dmg(
                    0.13 * vel.0.length(),
                    victim.inv.slot(5).item,
                    victim.inv.slot(6).item,
                    victim.inv.slot(7).item,
                    victim.inv.slot(8).item
                );

                damage_player(
                    &mut attacker,
                    &mut victim,
                    dmg,
                    Vec3::from(vel.0).normalize().with_y(0.0) * 0.6 * 20.0, // TODO: Make the knockback accurate
                    &mut deaths,
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
    positions: Query<(Entity, &Position, &PlayerGameState), With<Client>>,
    mut deaths: EventWriter<DeathEvent>,
) {
    for (entity, pos, gamestate) in positions.iter() {
        if pos.0.y < 75.0 {
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
            &mut CombatState,
        ),
        With<Client>,
    >,
    usernames: Query<&Username, With<Client>>,
    games: Query<&MapIndex>,
    mut deaths: EventReader<DeathEvent>,
    mut broadcasts: EventWriter<MessageEvent>,
    config: Res<BridgeConfig>,
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
            mut combatstate,
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
                            }) + if let Some(attacker) = combatstate.last_attacker {
                                if let Ok(username) = usernames.get(attacker) {
                                    Text::from(" was killed by ").color(Color::GRAY)
                                        + Text::from(username.0.clone()).color(if gamestate.team == 0 {
                                            Color::RED
                                        } else {
                                            Color::BLUE
                                        })
                                } else {
                                    Text::from(" has died!").color(Color::GRAY)
                                }
                            } else {
                                Text::from(" has died!").color(Color::GRAY)
                            },
                        });
                    }
                    combatstate.last_attacker = None;
                }
            }
        }
    }
}

fn play_death_sound(
    mut clients: Query<(&mut Client, &Position)>,
    states: Query<&CombatState>,
    mut deaths: EventReader<DeathEvent>
) {
    for DeathEvent(entity, show) in deaths.read() {
        let Ok(state) = states.get(*entity) else {
            continue;
        };
        let Some(attacker) = state.last_attacker else {
            continue;
        };
        if let Ok((mut client, pos)) = clients.get_mut(attacker) {
            if *show {
                client.play_sound(
                    Sound::EntityArrowHitPlayer,
                    SoundCategory::Player,
                    pos.0,
                    1.0,
                    1.0,
                );
            }
        }
    }
}

fn handle_score(
    clients: Query<(&Username, &PlayerGameState), With<Client>>,
    mut games: Query<(&Entities, &mut GameStage, &mut GameTime, &mut GameData)>,
    mut scores: EventReader<ScoreEvent>,
    mut deaths: EventWriter<DeathEvent>,
    mut broadcasts: EventWriter<MessageEvent>,
    mut gamestage: EventWriter<GameStageEvent>,
    mut end_game: EventWriter<EndGameEvent>,
) {
    for ScoreEvent(player) in scores.read() {
        let Ok((username, gamestate)) = clients.get(*player) else {
            continue;
        };
        let Some(game) = gamestate.game_id else {
            continue;
        };
        let Ok((entities, mut stage, mut time, mut data)) = games.get_mut(game) else {
            continue;
        };
        let team = gamestate.team as usize;
        let mut score = 0;
        if let Some(DataValue::Int(old_score)) = data.0.get(&(team)) {
            score = *old_score + 1;
        }
        data.0.insert(team, DataValue::Int(score));
        for entity in entities.0.iter() {
            deaths.send(DeathEvent(*entity, false));
        }
        broadcasts.send(MessageEvent {
            game: game,
            msg: Text::from(username.0.clone()).color(
                if team == 0 {
                    Color::BLUE
                } else {
                    Color::RED
                },
            ) + Text::from(" scored! (").color(Color::GRAY)
                + Text::from(score.to_string()).color(Color::GOLD)
                + Text::from("/5)").color(Color::GRAY),
        });
        if score >= 5 {
            end_game.send(EndGameEvent {
                game_id: game,
                loser: if team == 0 { 1 } else { 0 },
            });
        } else {
            time.0 = SystemTime::now(); // TODO: Replace with a better solution because the game time is not supposed to keep track of the entire game duration
            stage.0 = 0;
            gamestage.send(GameStageEvent {
                game_id: game,
                stage: 0,
            });
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

    victim.state.last_attacker = Some(attacker.entity);

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