use std::ops::RangeBounds;
use valence::prelude::*;
use crate::death::DeathEvent;
use crate::duels::{EndGameEvent, PlayerGameState};

pub enum OobMode {
    DeathEvent,
    GameEndEvent,
}

#[derive(Resource)]
struct OobResource<R> where
    R: RangeBounds<f64> + Send + Sync + Clone + 'static,
{
    pub bounds_y: R,
}

pub struct OobPlugin<R> where
    R: RangeBounds<f64> + Send + Sync + Clone + 'static,
{
    pub mode: OobMode,
    pub bounds_y: R,
}

impl <R> Plugin for OobPlugin<R> where
    R: RangeBounds<f64> + Send + Sync + Clone + 'static,
{
    fn build(&self, app: &mut App) {
        app.insert_resource(OobResource {
            bounds_y: self.bounds_y.clone(),
        });

        match self.mode {
            OobMode::DeathEvent => app.add_systems(Update, handle_oob_clients_death::<R>),
            OobMode::GameEndEvent => app.add_systems(Update, handle_oob_clients_end_game::<R>),
        };
    }
}

fn handle_oob_clients_death<R>(
    positions: Query<(Entity, &Position, &PlayerGameState), With<Client>>,
    mut deaths: EventWriter<DeathEvent>,
    oob: Res<OobResource<R>>,
) where
    R: RangeBounds<f64> + Send + Sync + Clone + 'static,
{
    for (entity, pos, gamestate) in positions.iter() {
        if !oob.bounds_y.contains(&pos.y) && gamestate.game_id.is_some() {
            deaths.send(DeathEvent(entity, true));
        }
    }
}

fn handle_oob_clients_end_game<R>(
    positions: Query<(&Position, &PlayerGameState), With<Client>>,
    mut end_game: EventWriter<EndGameEvent>,
    oob: Res<OobResource<R>>,
) where
    R: RangeBounds<f64> + Send + Sync + Clone + 'static,
{
    for (pos, gamestate) in positions.iter() {
        if !oob.bounds_y.contains(&pos.y) && let Some(game_id) = gamestate.game_id {
            end_game.send(EndGameEvent {
                game_id,
                loser: gamestate.team,
            });
        }
    }
}
