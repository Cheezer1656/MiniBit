#![allow(clippy::type_complexity)]

use std::collections::VecDeque;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use valence::prelude::*;
use valence::protocol::sound::{Sound, SoundCategory};
use valence::spawn::IsFlat;

const START_POS: BlockPos = BlockPos::new(0, 100, 0);
const VIEW_DIST: u8 = 10;

const BLOCK_TYPES: [BlockState; 7] = [
    BlockState::GRASS_BLOCK,
    BlockState::OAK_LOG,
    BlockState::BIRCH_LOG,
    BlockState::OAK_LEAVES,
    BlockState::BIRCH_LEAVES,
    BlockState::DIRT,
    BlockState::MOSS_BLOCK,
];

pub fn main() {
    let Ok(config) = std::fs::read_to_string("config.json") else {
        eprintln!("Failed to read `config.json`. Exiting.");
        return;
    };
    let Ok(config) = json::parse(&config) else {
        eprintln!("Failed to parse `config.json`. Exiting.");
        return;
    };

    if config["server"].is_null() {
        eprintln!("`server` or `world` key is missing in `config.json`. Exiting.");
        return;
    }

    App::new()
        .insert_resource(NetworkSettings {
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
        })
        .add_plugins(DefaultPlugins)
        .add_systems(
            Update,
            (
                init_clients,
                reset_clients.after(init_clients),
                manage_chunks.after(reset_clients).before(manage_blocks),
                manage_blocks,
                despawn_disconnected_clients,
            ),
        )
        .run();
}

#[derive(Component)]
struct GameState {
    blocks: VecDeque<BlockPos>,
    score: u32,
    combo: u32,
    target_y: i32,
    last_block_timestamp: u128,
}

fn init_clients(
    mut clients: Query<
        (
            Entity,
            &mut Client,
            &mut VisibleChunkLayer,
            &mut IsFlat,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    mut commands: Commands,
) {
    for (entity, mut client, mut visible_chunk_layer, mut is_flat, mut game_mode) in &mut clients {
        visible_chunk_layer.0 = entity;
        is_flat.0 = true;
        *game_mode = GameMode::Adventure;

        client.send_chat_message("Welcome to infinite parkour!".italic());

        let state = GameState {
            blocks: VecDeque::new(),
            score: 0,
            combo: 0,
            target_y: 0,
            last_block_timestamp: 0,
        };

        let layer = ChunkLayer::new(ident!("overworld"), &dimensions, &biomes, &server);

        commands.entity(entity).insert((state, layer));
    }
}

fn reset_clients(
    mut clients: Query<(
        &mut Client,
        &mut Position,
        &mut Look,
        &mut GameState,
        &mut ChunkLayer,
    )>,
) {
    for (mut client, mut pos, mut look, mut state, mut layer) in &mut clients {
        let out_of_bounds = (pos.0.y as i32) < START_POS.y - 32;

        if out_of_bounds || state.is_added() {
            if out_of_bounds && !state.is_added() {
                client.send_chat_message(
                    "Your score was ".italic()
                        + state
                            .score
                            .to_string()
                            .color(Color::GOLD)
                            .bold()
                            .not_italic(),
                );
            }

            // Init chunks.
            for pos in ChunkView::new(START_POS.into(), VIEW_DIST).iter() {
                layer.insert_chunk(pos, UnloadedChunk::new());
            }

            state.score = 0;
            state.combo = 0;

            for block in &state.blocks {
                layer.set_block(*block, BlockState::AIR);
            }
            state.blocks.clear();
            state.blocks.push_back(START_POS);
            layer.set_block(START_POS, BlockState::STONE);

            for _ in 0..10 {
                generate_next_block(&mut state, &mut layer, false);
            }

            pos.set([
                f64::from(START_POS.x) + 0.5,
                f64::from(START_POS.y) + 1.0,
                f64::from(START_POS.z) + 0.5,
            ]);
            look.yaw = 0.0;
            look.pitch = 0.0;
        }
    }
}

fn manage_blocks(mut clients: Query<(&mut Client, &Position, &mut GameState, &mut ChunkLayer)>) {
    for (mut client, pos, mut state, mut layer) in &mut clients {
        let pos_under_player = BlockPos::new(
            (pos.0.x - 0.5).round() as i32,
            pos.0.y as i32 - 1,
            (pos.0.z - 0.5).round() as i32,
        );

        if let Some(index) = state
            .blocks
            .iter()
            .position(|block| *block == pos_under_player)
        {
            if index > 0 {
                let power_result = 2_f32.powf((state.combo as f32) / 45.0);
                let max_time_taken = (1000_f32 * (index as f32) / power_result) as u128;

                let current_time_millis = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis();

                if current_time_millis - state.last_block_timestamp < max_time_taken {
                    state.combo += index as u32
                } else {
                    state.combo = 0
                }

                for _ in 0..index {
                    generate_next_block(&mut state, &mut layer, true)
                }

                let pitch = 0.9 + ((state.combo as f32) - 1.0) * 0.05;
                client.play_sound(
                    Sound::BlockNoteBlockBass,
                    SoundCategory::Master,
                    pos.0,
                    1.0,
                    pitch,
                );

                client.set_title("");
                client.set_subtitle(state.score.to_string().color(Color::LIGHT_PURPLE).bold());
            }
        }
    }
}

fn manage_chunks(mut clients: Query<(&Position, &OldPosition, &mut ChunkLayer), With<Client>>) {
    for (pos, old_pos, mut layer) in &mut clients {
        let old_view = ChunkView::new(old_pos.get().into(), VIEW_DIST);
        let view = ChunkView::new(pos.0.into(), VIEW_DIST);

        if old_view != view {
            for pos in old_view.diff(view) {
                layer.remove_chunk(pos);
            }

            for pos in view.diff(old_view) {
                layer.chunk_entry(pos).or_default();
            }
        }
    }
}

fn generate_next_block(state: &mut GameState, layer: &mut ChunkLayer, in_game: bool) {
    if in_game {
        let removed_block = state.blocks.pop_front().unwrap();
        layer.set_block(removed_block, BlockState::AIR);

        state.score += 1
    }

    let last_pos = *state.blocks.back().unwrap();
    let block_pos = generate_random_block(last_pos, state.target_y);

    if last_pos.y == START_POS.y {
        state.target_y = 0
    } else if last_pos.y < START_POS.y - 30 || last_pos.y > START_POS.y + 30 {
        state.target_y = START_POS.y;
    }

    layer.set_block(block_pos, BLOCK_TYPES[fastrand::usize(..BLOCK_TYPES.len())]);
    state.blocks.push_back(block_pos);

    // Combo System
    state.last_block_timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
}

fn generate_random_block(pos: BlockPos, target_y: i32) -> BlockPos {
    // if above or below target_y, change y to gradually reach it
    let y = match target_y {
        0 => fastrand::i32(-1..2),
        y if y > pos.y => 1,
        _ => -1,
    };
    let z = match y {
        1 => fastrand::i32(1..3),
        -1 => fastrand::i32(2..5),
        _ => fastrand::i32(1..4),
    };
    let x = fastrand::i32(-3..4);

    BlockPos::new(pos.x + x, pos.y + y, pos.z + z)
}