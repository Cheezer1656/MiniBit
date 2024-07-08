use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};

use valence::{math::DVec3, network::NetworkSettings, prelude::Resource, prelude::*};

#[derive(Resource)]
pub struct ServerConfig {
    pub world_paths: Vec<PathBuf>,
    pub spawns: Vec<Vec<DVec3>>,
}

pub fn load_config() -> Result<(NetworkSettings, ServerConfig), &'static str> {
    let Ok(config) = std::fs::read_to_string("config.json") else {
        return Err("Failed to read `config.json`. Exiting.");
    };
    let Ok(config) = json::parse(&config) else {
        return Err("Failed to parse `config.json`. Exiting.");
    };

    if config["server"].is_null() || config["worlds"].is_null() {
        return Err("`server` or `world` key is missing in `config.json`. Exiting.");
    }

    let world_paths = config["worlds"]
        .members()
        .map(|v| PathBuf::from(v["path"].as_str().unwrap_or("")))
        .collect::<Vec<PathBuf>>();

    let spawns = config["worlds"]
        .members()
        .map(|v| {
            v["spawns"]
                .members()
                .map(|v| DVec3::new(v[0].as_f64().unwrap_or(0.0), v[1].as_f64().unwrap_or(0.0), v[2].as_f64().unwrap_or(0.0)))
                .collect::<Vec<DVec3>>()
        })
        .collect::<Vec<Vec<DVec3>>>();

    for world_path in world_paths.iter() {
        if !world_path.exists() {
            return Err("One of the world paths does not exist. Exiting.");
        } else if !world_path.is_dir() {
            return Err("One of the world paths is not a directory. Exiting.");
        }
    }

    Ok((
        NetworkSettings {
            address: SocketAddr::new(
                config["server"]["ip"]
                    .as_str()
                    .unwrap_or("0.0.0.0")
                    .parse()
                    .unwrap_or(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))),
                config["server"]["port"].as_u16().unwrap_or(25565),
            ),
            max_players: config["server"]["max_players"].as_usize().unwrap_or(20),
            max_connections: config["server"]["max_players"].as_usize().unwrap_or(20),
            connection_mode: match config["server"]["connection_mode"].as_u8().unwrap_or(0) {
                1 => ConnectionMode::Offline,
                2 => ConnectionMode::BungeeCord,
                3 => ConnectionMode::Velocity {
                    secret: Arc::from(config["server"]["secret"].as_str().unwrap_or("")),
                },
                _ => ConnectionMode::Online {
                    prevent_proxy_connections: config["server"]["prevent_proxy_connections"]
                        .as_bool()
                        .unwrap_or(true),
                },
            },
            ..Default::default()
        },
        ServerConfig {
            world_paths,
            spawns,
        },
    ))
}
