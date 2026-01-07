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

#[allow(private_interfaces)]
pub mod models;
pub mod schema;

use diesel::prelude::*;
use std::sync::Mutex;
use valence::prelude::*;

#[derive(Resource)]
pub struct Database(pub Mutex<PgConnection>);

pub struct DatabasePlugin {
    connection_string: &'static str,
}

impl DatabasePlugin {
    pub fn new(connection_string: &'static str) -> Self {
        Self { connection_string }
    }
}

impl Plugin for DatabasePlugin {
    fn build(&self, app: &mut App) {
        let db = PgConnection::establish(self.connection_string).unwrap();
        app.insert_resource(Database(Mutex::new(db)));
    }
}
