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
