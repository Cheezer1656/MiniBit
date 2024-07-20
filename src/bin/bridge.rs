#![allow(clippy::type_complexity)]

#[path = "../lib/mod.rs"]
mod lib;

use bevy_ecs::query::WorldQuery;
use lib::duels::*;
use valence::entity::living::Health;
use valence::entity::{EntityId, EntityStatuses};
use valence::math::Vec3Swizzles;
use valence::prelude::*;
use valence::protocol::packets::play::DamageTiltS2c;
use valence::protocol::sound::SoundCategory;
use valence::protocol::Sound;
use valence::protocol::VarInt;
use valence::protocol::WritePacket;

#[derive(Event)]
struct DeathEvent(Entity);

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

fn main() {
    App::new()
        .add_plugins(DuelsPlugin {
            default_gamemode: GameMode::Survival,
        })
        .add_plugins(DefaultPlugins)
        .add_event::<DeathEvent>()
        .add_event::<ScoreEvent>()
        .add_event::<MessageEvent>()
        .add_systems(EventLoopUpdate, handle_combat_events)
        .add_systems(
            Update,
            (start_game, gamestage_change, end_game, check_goals, handle_death, handle_score.after(check_goals), handle_oob_clients, game_broadcast),
        )
        .run();
}

fn start_game(
    mut games: Query<&mut GameData>,
    mut start_game: EventReader<StartGameEvent>,
) {
    for event in start_game.read() {
        if let Ok(mut data) = games.get_mut(event.0) {
            data.0.insert(0, DataValue::Int(0));
            data.0.insert(1, DataValue::Int(0));
        }
    }
}

fn fill_inventory(inv: &mut Inventory, team: u8) {
    let block_type = match team {
        0 => ItemKind::BlueTerracotta,
        1 => ItemKind::RedTerracotta,
        _ => ItemKind::Terracotta,
    };
    inv.set_slot(36, ItemStack::new(ItemKind::IronSword, 1, None));
    inv.set_slot(37, ItemStack::new(ItemKind::Bow, 1, None));
    inv.set_slot(38, ItemStack::new(ItemKind::DiamondPickaxe, 1, None));
    inv.set_slot(39, ItemStack::new(block_type, 64, None));
    inv.set_slot(40, ItemStack::new(block_type, 64, None));
    inv.set_slot(41, ItemStack::new(ItemKind::GoldenApple, 8, None));
    inv.set_slot(44, ItemStack::new(ItemKind::Arrow, 10, None));
}

fn gamestage_change(
    mut clients: Query<(&mut Inventory, &PlayerGameState), With<Client>>,
    games: Query<&Entities>,
    mut event: EventReader<GameStageEvent>,
) {
    for event in event.read() {
        if event.stage != 4 {
            continue;
        }
        if let Ok(entities) = games.get(event.game_id) {
            for entity in entities.0.iter() {
                if let Ok((mut inventory, gamestate)) = clients.get_mut(*entity) {
                    fill_inventory(&mut inventory, gamestate.team);
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

// TODO - Fix double scoring bug
fn check_goals(
    clients: Query<(&Position, &PlayerGameState), With<Client>>,
    config: Res<DuelsConfig>,
    mut scores: EventWriter<ScoreEvent>,
) {
    for (pos, gamestate) in clients.iter() {
        if let Some(game_id) = gamestate.game_id {
            if let Some(data) = &config.other {
                let x = pos.0.x.floor() as isize;
                let z = pos.0.z.floor() as isize;
                if data[0] <= x && data[1] >= x
                    && pos.0.y.floor() as isize == data[2]
                    && data[3] <= z && data[4] >= z {
                    scores.send(ScoreEvent {
                        game: game_id,
                        team: gamestate.team,
                    });
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

fn handle_oob_clients(
    positions: Query<(Entity, &Position, &PlayerGameState), With<Client>>,
    mut deaths: EventWriter<DeathEvent>,
) {
    for (entity, pos, gamestate) in positions.iter() {
        if pos.0.y < 0.0 {
            if gamestate.game_id.is_some() {
                deaths.send(DeathEvent(entity));
            }
        }
    }
}

fn handle_death(
    mut clients: Query<(&mut Position, &mut Look, &mut HeadYaw, &mut Health, &mut Inventory, &Username, &PlayerGameState), With<Client>>,
    games: Query<&MapIndex>,
    mut deaths: EventReader<DeathEvent>,
    mut broadcasts: EventWriter<MessageEvent>,
    config: Res<DuelsConfig>,
) {
    for DeathEvent(entity) in deaths.read() {
        if let Ok((mut pos, mut look, mut head_yaw, mut health, mut inventory, username, gamestate)) = clients.get_mut(*entity) {
            if let Some(game_id) = gamestate.game_id {
                if let Ok(map_index) = games.get(game_id) {
                    let spawn = &config.worlds[map_index.0].spawns[gamestate.team as usize];
                    pos.0 = spawn.pos.into();
                    look.yaw = spawn.rot[0];
                    look.pitch = spawn.rot[1];
                    head_yaw.0 = spawn.rot[0];
                    health.0 = 20.0;
                    for slot in 0..inventory.slot_count() {
                        inventory.set_slot(slot, ItemStack::EMPTY);
                    }
                    fill_inventory(&mut inventory, gamestate.team);
                    broadcasts.send(MessageEvent {
                        game: game_id,
                        msg: (username.0.clone() + " has died!").into(),
                    });
                }
            }
        }
    }
}

fn handle_score(
    mut games: Query<(&Entities, &mut GameData)>,
    mut scores: EventReader<ScoreEvent>,
    mut deaths: EventWriter<DeathEvent>,
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
                deaths.send(DeathEvent(*entity));
            }
            println!("Score: {}", score);
            if score >= 5 {
                end_game.send(EndGameEvent { game_id: *game, loser: if *team == 0 { 1 } else { 0 } });
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
