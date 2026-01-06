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

use std::{env, marker::PhantomData, net::{IpAddr, SocketAddr}, path::PathBuf, str::FromStr, sync::Arc};
use ::serde::Deserialize;
use serde::de::DeserializeOwned;
use valence::{network::NetworkSettings, prelude::*, CompressionThreshold, ServerSettings};

#[derive(Deserialize)]
pub struct NetworkConfig {
    pub ip: String,
    pub port: u16,
    pub max_players: usize,
    pub connection_mode: u8,
    pub prevent_proxy_connections: bool,
}

#[derive(Resource, Deserialize)]
pub struct EmptyConfig {}

#[derive(Deserialize)]
pub struct WorldValue {
    pub path: String,
    pub x_chunks: [i32; 2],
    pub z_chunks: [i32; 2],
    pub spawns: Vec<SpawnValue>,
}

#[derive(Deserialize)]
pub struct SpawnValue {
    pub pos: [f64; 3],
    pub rot: [f32; 2],
}

#[derive(Resource)]
pub struct DataPath(pub PathBuf);

pub struct ConfigLoaderPlugin<T: DeserializeOwned> {
    pub path: PathBuf,
    pub phantom: PhantomData<T>,
}

impl<T: Resource + DeserializeOwned + Sync + Send + 'static> Plugin for ConfigLoaderPlugin<T> {
    fn build(&self, app: &mut App) {
        let data = std::fs::read_to_string(self.path.join("server.json")).unwrap();
        let netconfig = serde_json::from_str::<NetworkConfig>(&data).unwrap();

        let secret = if netconfig.connection_mode == 3 {
            env::var("FORWARDING_SECRET").expect("Failed to read secret env var")
        } else {
            String::new()
        };

        let data = std::fs::read_to_string(self.path.join("config.json")).unwrap();
        let config = serde_json::from_str::<T>(&data).unwrap();

        app.insert_resource(ServerSettings {
            compression_threshold: CompressionThreshold(-1),
            ..Default::default()
        })
        .insert_resource(NetworkSettings {
            address: SocketAddr::new(IpAddr::from_str(&netconfig.ip).unwrap(), netconfig.port),
            max_players: netconfig.max_players,
            connection_mode: match netconfig.connection_mode {
                1 => ConnectionMode::Offline,
                2 => ConnectionMode::BungeeCord,
                3 => ConnectionMode::Velocity {
                    secret: Arc::from(secret),
                },
                _ => ConnectionMode::Online {
                    prevent_proxy_connections: netconfig.prevent_proxy_connections,
                },
            },
            ..Default::default()
        })
        .insert_resource(config)
        .insert_resource(DataPath(self.path.clone()));
    }
}
