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

pub mod map;
pub mod copied_map;

use bevy_ecs::query::QueryData;
use serde::Deserialize;
use std::{collections::HashMap, i64, marker::PhantomData, time::SystemTime};
use valence::{
    entity::living::Health,
    message::ChatMessageEvent,
    prelude::*,
    protocol::{sound::SoundCategory, Sound},
};

use super::config::{ConfigLoaderPlugin, WorldValue};

#[derive(Component)]
pub struct MapIndex(pub usize);

#[derive(Component)]
pub struct Entities(pub Vec<Entity>);

#[derive(Component)]
pub struct GameTime(pub SystemTime);

#[derive(Component)]
pub struct GameStage(pub u8);

pub enum DataValue {
    Int(i32),
    Float(f32),
    String(String),
}

#[derive(Component)]
pub struct GameData(pub HashMap<usize, DataValue>);

#[derive(Component, Default)]
pub struct PlayerGameState {
    pub game_id: Option<Entity>,
    pub team: u8,
    pub wins: u32,
}

#[derive(Component, Default)]
pub struct CombatState {
    pub last_attacked_tick: i64,
    pub has_bonus_knockback: bool,
}

#[derive(Event)]
pub struct StartGameEvent(pub Entity);

#[derive(Event)]
pub struct EndGameEvent {
    pub game_id: Entity,
    pub loser: u8,
}

#[derive(Event)]
pub struct GameStageEvent {
    pub game_id: Entity,
    pub stage: u8,
}

#[derive(Resource)]
pub struct ServerGlobals {
    pub queue: Vec<Entity>,
}

#[derive(Resource)]
pub struct GameSettings {
    pub default_gamemode: GameMode,
}

#[derive(Resource, Deserialize)]
pub struct DuelsConfig {
    pub worlds: Vec<WorldValue>,
    pub other: Option<Vec<isize>>,
}

pub struct DuelsPlugin {
    pub default_gamemode: GameMode,
    pub copy_map: bool,
}

impl Plugin for DuelsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ConfigLoaderPlugin::<DuelsConfig> {
            phantom: PhantomData,
        })
        .insert_resource(GameSettings {
            default_gamemode: self.default_gamemode,
        })
        .insert_resource(ServerGlobals { queue: Vec::new() })
        .add_event::<StartGameEvent>()
        .add_event::<EndGameEvent>()
        .add_event::<GameStageEvent>()
        .add_systems(
            Update,
            (
                despawn_disconnected_clients,
                start_game,
                gamestage_change.after(gameloop),
                chat_message,
            ),
        )
        .add_systems(PostUpdate, handle_disconnect);

        if self.copy_map {
            app
                .add_plugins(copied_map::MapPlugin)
                .add_systems(Update, gameloop.after(map::start_game));
        } else {
            app
                .add_plugins(map::MapPlugin)
                .add_systems(Update, gameloop);
        }
    }
}

pub fn handle_disconnect(
    disconncted: Query<(Entity, &PlayerGameState), Added<Despawned>>,
    mut clients: Query<(&mut Client, &PlayerGameState)>,
    mut end_game: EventWriter<EndGameEvent>,
    mut globals: ResMut<ServerGlobals>,
) {
    for (entity, dc_gamestate) in disconncted.iter() {
        if globals.queue.contains(&entity) {
            globals.queue.retain(|&x| x != entity);
        } else {
            for (mut client, gamestate) in clients.iter_mut() {
                if gamestate.game_id == dc_gamestate.game_id {
                    client.send_chat_message("Your opponent disconnected!");
                    client.clear_title();
                }
            }
            end_game.send(EndGameEvent {
                game_id: dc_gamestate.game_id.unwrap(),
                loser: dc_gamestate.team,
            });
        }
    }
}

pub fn start_game(
    mut start_game: EventReader<StartGameEvent>,
    mut gamestage: EventWriter<GameStageEvent>,
) {
    for event in start_game.read() {
        gamestage.send(GameStageEvent {
            game_id: event.0,
            stage: 0,
        });
    }
}

#[derive(QueryData)]
#[query_data(mutable)]
pub struct GameQuery {
    client: &'static mut Client,
    gamestate: &'static PlayerGameState,
    pos: &'static mut Position,
    look: &'static mut Look,
    yaw: &'static mut HeadYaw,
}

pub fn gameloop(
    mut clients: Query<GameQuery>,
    mut games: Query<(Entity, &Entities, &MapIndex, &mut GameStage, &GameTime)>,
    mut gamestage: EventWriter<GameStageEvent>,
    config: Res<DuelsConfig>,
) {
    for (game_id, entities, map, mut stage, time) in games.iter_mut() {
        if stage.0 < 4 {
            for entity in entities.0.iter() {
                if let Ok(mut player) = clients.get_mut(*entity) {
                    let spawn = &config.worlds[map.0].spawns[player.gamestate.team as usize % 2];
                    player.pos.set(spawn.pos);
                    player.look.yaw = spawn.rot[0];
                    player.look.pitch = spawn.rot[1];
                    player.yaw.0 = spawn.rot[0];
                }
            }
        }
        if (stage.0 < 5 && time.0.elapsed().unwrap().as_secs() >= stage.0 as u64) || stage.0 == 0 {
            stage.0 += 1;
            gamestage.send(GameStageEvent {
                game_id,
                stage: stage.0,
            });
        }
    }
}

pub fn gamestage_change(
    mut clients: Query<(&mut Client, &Position)>,
    games: Query<&Entities>,
    mut gamestage: EventReader<GameStageEvent>,
) {
    for event in gamestage.read() {
        if let Ok(entities) = games.get(event.game_id) {
            for entity in entities.0.iter() {
                if let Ok((mut client, pos)) = clients.get_mut(*entity) {
                    match event.stage {
                        1 => client.set_title("3".color(Color::GREEN)),
                        2 => client.set_title("2".color(Color::GOLD)),
                        3 => client.set_title("1".color(Color::RED)),
                        4 => client.set_title("GO!".color(Color::RED)),
                        5 => client.clear_title(),
                        _ => {}
                    }
                    if event.stage < 4 {
                        client.play_sound(
                            Sound::BlockNoteBlockPling,
                            SoundCategory::Master,
                            pos.0,
                            1.0,
                            1.0,
                        );
                    } else if event.stage == 4 {
                        client.play_sound(
                            Sound::BlockNoteBlockPling,
                            SoundCategory::Master,
                            pos.0,
                            1.0,
                            5.0,
                        );
                    }
                }
            }
        }
    }
}

pub fn chat_message(
    players: Query<(&PlayerGameState, &Username)>,
    mut clients: Query<(&mut Client, &PlayerGameState)>,
    mut events: EventReader<ChatMessageEvent>,
) {
    for event in events.read() {
        let Ok((sender_gamestate, sender_name)) = players.get(event.client) else {
            continue;
        };
        for (mut client, gamestate) in clients.iter_mut() {
            if gamestate.game_id == sender_gamestate.game_id {
                client.send_chat_message(
                    (String::new() + &sender_name.0 + &String::from(": ") + &event.message)
                        .color(Color::GRAY),
                );
            }
        }
    }
}
