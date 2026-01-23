use valence::{
    command::{CommandScopeRegistry, scopes::CommandScopes},
    prelude::*,
};

pub struct ScopePlugin;

impl Plugin for ScopePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, add_default_scope);
    }
}

fn setup(mut command_scopes: ResMut<CommandScopeRegistry>) {
    command_scopes.link("minibit.all", "minibit.commands.all");
}

fn add_default_scope(mut clients: Query<&mut CommandScopes, Added<Client>>) {
    for mut scopes in clients.iter_mut() {
        scopes.add("minibit.all");
    }
}
