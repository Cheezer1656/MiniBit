use ::serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::{
    marker::PhantomData,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};
use valence::{CompressionThreshold, ServerSettings, network::NetworkSettings, prelude::*};

#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct NetworkConfig {
    pub ip: String,
    pub port: u16,
    pub max_players: usize,

    pub prevent_proxy_connections: bool,

    #[serde(skip_deserializing)]
    pub connection_mode: u8,
    #[serde(skip_deserializing)]
    pub forwarding_secret: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        NetworkConfig {
            ip: "0.0.0.0".to_string(),
            port: 25565,
            max_players: 100,
            connection_mode: 1,
            prevent_proxy_connections: false,
            forwarding_secret: "".to_string(),
        }
    }
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
    pub network_config: NetworkConfig,
    pub phantom: PhantomData<T>,
}

impl<T: Resource + DeserializeOwned + Sync + Send + 'static> Plugin for ConfigLoaderPlugin<T> {
    fn build(&self, app: &mut App) {
        let data = std::fs::read_to_string(self.path.join("config.json")).unwrap();
        let config = serde_json::from_str::<T>(&data).unwrap();

        app.insert_resource(ServerSettings {
            compression_threshold: CompressionThreshold(-1),
            ..Default::default()
        })
        .insert_resource(NetworkSettings {
            address: SocketAddr::new(
                IpAddr::from_str(&self.network_config.ip).unwrap(),
                self.network_config.port,
            ),
            max_players: self.network_config.max_players,
            connection_mode: match self.network_config.connection_mode {
                1 => ConnectionMode::Offline,
                2 => ConnectionMode::BungeeCord,
                3 => ConnectionMode::Velocity {
                    secret: Arc::from(self.network_config.forwarding_secret.clone()),
                },
                _ => ConnectionMode::Online {
                    prevent_proxy_connections: self.network_config.prevent_proxy_connections,
                },
            },
            ..Default::default()
        })
        .insert_resource(config)
        .insert_resource(DataPath(self.path.clone()));
    }
}
