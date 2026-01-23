use valence::{
    command::{handler::CommandResultEvent, AddCommand},
    command_macros::Command,
    prelude::*,
};

use crate::lobby::LobbyConfig;

#[derive(Command, Debug, Clone)]
#[paths("stuck")]
#[scopes("minibit.commands.all.stuck")]
struct StuckCommand {}

pub struct CommandPlugin;

impl Plugin for CommandPlugin {
    fn build(&self, app: &mut App) {
        app.add_command::<StuckCommand>()
            .add_systems(Update, handle_stuck_command);
    }
}

fn handle_stuck_command(
    mut events: EventReader<CommandResultEvent<StuckCommand>>,
    mut clients: Query<(Entity, &mut Position, &mut Look, &mut HeadYaw), With<Client>>,
    config: Res<LobbyConfig>,
) {
    for event in events.read() {
        if let Ok((_, mut pos, mut look, mut head_yaw)) = clients.get_mut(event.executor) {
            pos.set(config.world.spawns[0].pos);
            look.yaw = config.world.spawns[0].rot[0];
            look.pitch = config.world.spawns[0].rot[1];
            head_yaw.0 = config.world.spawns[0].rot[0];
        }
    }
}
