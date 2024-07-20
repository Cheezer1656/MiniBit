use std::{
    marker::PhantomData,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    sync::Arc,
};

use ::serde::Deserialize;
use serde::de::DeserializeOwned;
use valence::{network::NetworkSettings, prelude::*, CompressionThreshold, ServerSettings};

#[derive(Deserialize)]
pub struct NetworkConfig<'a> {
    pub ip: String,
    pub port: u16,
    pub max_players: usize,
    pub connection_mode: u8,
    pub secret: &'a str,
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

pub struct ConfigLoaderPlugin<T: DeserializeOwned> {
    pub phantom: PhantomData<T>,
}

impl<T: Resource + DeserializeOwned + Sync + Send + 'static> Plugin for ConfigLoaderPlugin<T> {
    fn build(&self, app: &mut App) {
        let data = std::fs::read_to_string("server.json").unwrap();

        let netconfig = serde_json::from_str::<NetworkConfig>(&data).unwrap();

        let data = std::fs::read_to_string("config.json").unwrap();

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
                    secret: Arc::from(netconfig.secret),
                },
                _ => ConnectionMode::Online {
                    prevent_proxy_connections: netconfig.prevent_proxy_connections,
                },
            },
            ..Default::default()
        })
        .insert_resource(config);
    }
}
