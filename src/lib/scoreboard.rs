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

use valence::{prelude::*, scoreboard::{Objective, ObjectiveBundle, ObjectiveDisplay, ObjectiveScores}};

use super::color::format;

#[derive(Component)]
pub struct ScoreboardId(pub Entity);

pub enum ScoreboardMode {
    ServerWide,
    PerPlayer,
}

#[derive(Resource)]
pub struct ScoreboardGlobals {
    pub layer: EntityLayerId,
}

#[derive(Resource)]
pub struct ScoreboardPluginResource {
    name: &'static str,
    text: Vec<String>,
}

pub struct ScoreboardPlugin {
    pub name: &'static str,
    pub text: Vec<String>,
    pub mode: ScoreboardMode,
}

impl Plugin for ScoreboardPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ScoreboardPluginResource {
            name: self.name,
            text: self.text.clone(),
        });

        match self.mode {
            ScoreboardMode::ServerWide => {
                app.add_systems(Startup, setup)
                    .add_systems(Update, init_clients_0);
            },
            ScoreboardMode::PerPlayer => {
                app.add_systems(Update, (init_clients_1, cleanup_clients));
            },
        }
    }
}

pub fn setup(mut commands: Commands, server: Res<Server>, res: Res<ScoreboardPluginResource>) {
    let obj_layer_id = commands.spawn(EntityLayer::new(&server)).id();
    commands.insert_resource(ScoreboardGlobals {
        layer: EntityLayerId(obj_layer_id),
    });
    let obj = ObjectiveBundle {
        name: Objective::new("sidebar"),
        display: ObjectiveDisplay(res.name.color(Color::GOLD).bold()),
        layer: EntityLayerId(obj_layer_id),
        scores: gen_scores(&res.text),
        ..Default::default()
    };
    commands.spawn(obj);
}

pub fn init_clients_0(mut clients: Query<&mut VisibleEntityLayers, Added<Client>>, globals: Res<ScoreboardGlobals>) {
    for mut layers in clients.iter_mut() {
        layers.0.insert(globals.layer.0);
    }
}

pub fn init_clients_1(
    mut commands: Commands,
    mut clients: Query<(Entity, &mut VisibleEntityLayers), Added<Client>>,
    server: Res<Server>,
    res: Res<ScoreboardPluginResource>,
) {
    for (entity, mut layers) in clients.iter_mut() {
        let layer = EntityLayer::new(&server);
        let obj = ObjectiveBundle {
            name: Objective::new("sidebar"),
            display: ObjectiveDisplay(res.name.color(Color::GOLD).bold()),
            layer: EntityLayerId(entity),
            scores: gen_scores(&res.text),
            ..Default::default()
        };
        let obj_id = commands.spawn(obj).id();
        commands.entity(entity).insert((layer, ScoreboardId(obj_id)));
        layers.0.insert(entity);
    }
}

pub fn cleanup_clients(
    mut commands: Commands,
    clients: Query<&ScoreboardId>,
    mut removed: RemovedComponents<Client>
) {
    for entity in removed.read() {
        if let Ok(ScoreboardId(obj_id)) = clients.get(entity) {
            commands.entity(*obj_id).despawn();
        }
    }
}

pub fn gen_scores(text: &Vec<String>) -> ObjectiveScores {
    let mut scores = ObjectiveScores::new();
    scores.insert(format::DARK_GRAY.to_string()+format::BOLD+format::STRIKETHROUGH+"-------------------", text.len() as i32 + 4);
    scores.insert("  ", text.len() as i32 + 3);
    scores.insert("", 2);
    scores.insert(format::DARK_GRAY.to_string()+format::BOLD+format::STRIKETHROUGH+"------------------ ", 1);
    scores.insert(format::YELLOW.to_string()+"minibit.net", 0);
    for (i, line) in text.iter().rev().enumerate() {
        let mut line = line.clone();
        while scores.get(&line).is_some() {
            line.push(' ');
        }
        scores.insert(line, 3+i as i32);
    }

    scores
}