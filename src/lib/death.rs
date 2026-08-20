use crate::duels::CombatState;
use chunkedge::prelude::*;
use chunkedge::protocol::Sound;
use chunkedge::protocol::sound::SoundCategory;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeathSet;

#[derive(Message)]
pub struct DeathMessage(pub Entity, pub bool);

pub struct DeathPlugin;

impl Plugin for DeathPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DeathMessage>()
            .add_systems(Update, play_death_sound.in_set(DeathSet));
    }
}

pub fn play_death_sound(
    mut clients: Query<(&mut Client, &Position)>,
    states: Query<&CombatState>,
    mut deaths: MessageReader<DeathMessage>,
) {
    for DeathMessage(entity, show) in deaths.read() {
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
