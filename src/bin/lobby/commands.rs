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

use valence::{
    command::{handler::CommandResultEvent, AddCommand},
    command_macros::Command,
    prelude::*,
};

use crate::LobbyConfig;

#[derive(Command, Debug, Clone)]
#[paths("stuck")]
#[scopes("minibit.commands.all.stuck")]
struct StuckCommand {}

pub struct CommandPlugin;

impl Plugin for CommandPlugin {
    fn build(&self, app: &mut App) {
        app.add_command::<StuckCommand>()
            .add_systems(Update, handle_stuck_command);
    }
}

fn handle_stuck_command(
    mut events: EventReader<CommandResultEvent<StuckCommand>>,
    mut clients: Query<(Entity, &mut Position, &mut Look, &mut HeadYaw), With<Client>>,
    config: Res<LobbyConfig>,
) {
    for event in events.read() {
        if let Ok((_, mut pos, mut look, mut head_yaw)) = clients.get_mut(event.executor) {
            pos.set(config.world.spawns[0].pos);
            look.yaw = config.world.spawns[0].rot[0];
            look.pitch = config.world.spawns[0].rot[1];
            head_yaw.0 = config.world.spawns[0].rot[0];
        }
    }
}
