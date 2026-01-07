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
    command::{CommandScopeRegistry, scopes::CommandScopes},
    prelude::*,
};

pub struct ScopePlugin;

impl Plugin for ScopePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, add_default_scope);
    }
}

fn setup(mut command_scopes: ResMut<CommandScopeRegistry>) {
    command_scopes.link("minibit.all", "minibit.commands.all");
}

fn add_default_scope(mut clients: Query<&mut CommandScopes, Added<Client>>) {
    for mut scopes in clients.iter_mut() {
        scopes.add("minibit.all");
    }
}
