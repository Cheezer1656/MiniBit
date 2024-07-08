#![allow(clippy::type_complexity)]

#[path = "../lib/mod.rs"]
mod lib;

use lib::config::*;
use lib::game::*;
use valence::{prelude::*, CompressionThreshold, ServerSettings};

pub fn main() {
    let config = match load_config() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("{}", e);
            return;
        }
    };

    App::new()
        .insert_resource(config.0)
        .insert_resource(ServerSettings {
            compression_threshold: CompressionThreshold(-1),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .insert_resource(config.1)
        .add_event::<StartGameEvent>()
        .add_event::<EndGameEvent>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                init_clients,
                despawn_disconnected_clients,
                handle_oob_clients,
                lib::game::start_game.after(init_clients),
                start_game.after(lib::game::start_game),
                lib::game::end_game.after(handle_oob_clients),
                end_game.after(lib::game::end_game),
                gameloop.after(start_game),
                chat_message,
            ),
        )
        .add_systems(PostUpdate, (handle_disconnect, check_queue))
        .run();
}

fn start_game(
    mut clients: Query<&mut Inventory, With<Client>>,
    games: Query<&Entities>,
    mut start_game: EventReader<StartGameEvent>,
) {
    for event in start_game.read() {
        if let Ok(entities) = games.get(event.0) {
            for entity in entities.0.iter() {
                if let Ok(mut inventory) = clients.get_mut(*entity) {
                    inventory.set_slot(36, ItemStack::new(ItemKind::DiamondShovel, 1, None));
                }
            }
        }
    }
}

fn end_game(
    mut clients: Query<&mut Inventory, With<Client>>,
    games: Query<&Entities>,
    mut end_game: EventReader<EndGameEvent>,
) {
    for event in end_game.read() {
        if let Ok(entities) = games.get(event.game_id) {
            for entity in entities.0.iter() {
                if let Ok(mut inv) = clients.get_mut(*entity) {
                    for slot in 0..inv.slot_count() {
                        inv.set_slot(slot, ItemStack::new(ItemKind::Air, 0, None));
                    }
                }
            }
        }
    }
}

fn handle_oob_clients(
    positions: Query<(&mut Position, &PlayerGameState), With<Client>>,
    mut end_game: EventWriter<EndGameEvent>,
) {
    for (pos, gamestate) in positions.iter() {
        if pos.0.y < 0.0 {
            if gamestate.game_id.is_some() {
                end_game.send(EndGameEvent {
                    game_id: gamestate.game_id.unwrap(),
                    loser: gamestate.team
                });
            }
        }
    }
}