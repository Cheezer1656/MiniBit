use crate::duels::CombatState;
use valence::prelude::*;
use valence::protocol::Sound;
use valence::protocol::sound::SoundCategory;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeathSet;

#[derive(Event)]
pub struct DeathEvent(pub Entity, pub bool);

pub struct DeathPlugin;

impl Plugin for DeathPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DeathEvent>()
            .add_systems(Update, play_death_sound.in_set(DeathSet));
    }
}

pub fn play_death_sound(
    mut clients: Query<(&mut Client, &Position)>,
    states: Query<&CombatState>,
    mut deaths: EventReader<DeathEvent>,
) {
    for DeathEvent(entity, show) in deaths.read() {
        let Ok(state) = states.get(*entity) else {
            continue;
        };
        let Some(attacker) = state.last_attacker else {
            continue;
        };
        if let Ok((mut client, pos)) = clients.get_mut(attacker)
            && *show
        {
            client.play_sound(
                Sound::EntityArrowHitPlayer,
                SoundCategory::Player,
                pos.0,
                1.0,
                1.0,
            );
        }
    }
}
