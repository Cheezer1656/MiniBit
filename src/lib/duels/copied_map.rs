/*
    MiniBit - A Minecraft minigame server network written in Rust.
    Copyright (C) 2024  Cheezer1656 (https://github.com/Cheezer1656/)

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published
    by the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use super::*;
use crate::config::DataPath;
use valence::prelude::*;
use valence_anvil::AnvilLevel;

#[derive(Bundle)]
pub struct Game {
    pub map_index: MapIndex,
    pub layer: EntityLayerId,
    pub clients: Entities,
    pub game_start: GameTime,
    pub game_stage: GameStage,
    pub data: GameData,
}

#[derive(Resource)]
pub struct MapGlobals {
    pub queue_layer: Entity,
}

pub struct MapPlugin<T: Resource + DuelsConfig> {
    pub phantom: PhantomData<T>,
}

impl<T: Resource + DeserializeOwned + DuelsConfig + Sync + Send + 'static> Plugin for MapPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup::<T>)
            .add_systems(Update, (init_clients::<T>,))
            .add_systems(PostUpdate, (check_queue::<T>, end_game::<T>));
    }
}

pub fn setup<T: Resource + DuelsConfig>(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    config: Res<T>,
    data_path: Res<DataPath>,
) {
    let layer_id = commands
        .spawn(init_world(
            &config.worlds()[0],
            &server,
            &dimensions,
            &biomes,
            &data_path,
        ))
        .id();

    commands.insert_resource(MapGlobals {
        queue_layer: layer_id,
    });
}

fn init_world(
    world: &WorldValue,
    server: &Res<Server>,
    dimensions: &Res<DimensionTypeRegistry>,
    biomes: &Res<BiomeRegistry>,
    data_path: &Res<DataPath>,
) -> (LayerBundle, AnvilLevel) {
    let layer = LayerBundle::new(ident!("overworld"), dimensions, biomes, server);
    let mut level = AnvilLevel::new(data_path.0.join(world.path.clone()), biomes);

    for z in world.z_chunks[0]..=world.z_chunks[1] {
        for x in world.x_chunks[0]..=world.x_chunks[1] {
            let pos = ChunkPos::new(x, z);

            level.ignored_chunks.insert(pos);
            level.force_chunk_load(pos);
        }
    }

    (layer, level)
}

pub fn init_clients<T: Resource + DuelsConfig>(
    mut clients: Query<
        (
            Entity,
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut GameMode,
            &mut Health,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
    mut commands: Commands,
    mut server_globals: ResMut<ServerGlobals>,
    globals: Res<MapGlobals>,
    settings: Res<GameSettings>,
    config: Res<T>,
) {
    for (
        entity,
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut game_mode,
        mut health,
    ) in clients.iter_mut()
    {
        let Ok(layer) = layers.get(globals.queue_layer) else {
            continue;
        };

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set(config.worlds()[0].spawns[0].pos);
        *game_mode = settings.default_gamemode;
        health.0 = 20.0;
        commands
            .entity(entity)
            .insert((PlayerGameState::default(), CombatState::default()));

        server_globals.queue.push(entity);
    }
}

pub fn check_queue<T: Resource + DuelsConfig>(
    mut clients: Query<(
        &mut Client,
        &mut PlayerGameState,
        &mut EntityLayerId,
        &mut VisibleChunkLayer,
        &mut VisibleEntityLayers,
        &mut Position,
        &mut Look,
        &mut HeadYaw,
    )>,
    mut start_game_ev: EventWriter<StartGameEvent>,
    config: Res<T>,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    data_path: Res<DataPath>,
    mut commands: Commands,
    mut server_globals: ResMut<ServerGlobals>,
) {
    if server_globals.queue.len() < 2 {
        return;
    }
    fastrand::shuffle(&mut server_globals.queue);
    while server_globals.queue.len() > 1 {
        start_game(
            server_globals.queue.drain(..2).collect(),
            &mut clients,
            &mut start_game_ev,
            &mut commands,
            &server,
            &dimensions,
            &biomes,
            &config,
            &data_path,
        );
    }
}

fn start_game<T: Resource + DuelsConfig>(
    entities: Vec<Entity>,
    clients: &mut Query<(
        &mut Client,
        &mut PlayerGameState,
        &mut EntityLayerId,
        &mut VisibleChunkLayer,
        &mut VisibleEntityLayers,
        &mut Position,
        &mut Look,
        &mut HeadYaw,
    )>,
    start_game_ev: &mut EventWriter<StartGameEvent>,
    commands: &mut Commands,
    server: &Res<Server>,
    dimensions: &Res<DimensionTypeRegistry>,
    biomes: &Res<BiomeRegistry>,
    config: &Res<T>,
    data_path: &Res<DataPath>,
) {
    let map_idx = fastrand::usize(1..config.worlds().len());
    let world = &config.worlds()[map_idx];
    let layer = commands
        .spawn(init_world(world, server, dimensions, biomes, data_path))
        .id();

    let game_id = commands
        .spawn(Game {
            map_index: MapIndex(map_idx),
            layer: EntityLayerId(layer),
            clients: Entities(entities.clone()),
            game_start: GameTime(SystemTime::now()),
            game_stage: GameStage(0),
            data: GameData(HashMap::new()),
        })
        .id();

    for (i, entity) in entities.iter().enumerate() {
        let Ok((
            mut client,
            mut gamestate,
            mut layer_id,
            mut visible_chunk_layer,
            mut visible_entity_layers,
            mut pos,
            mut look,
            mut headyaw,
        )) = clients.get_mut(*entity)
        else {
            continue;
        };

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.clear();
        visible_entity_layers.0.insert(layer);

        gamestate.game_id = Some(game_id);
        gamestate.team = i as u8 % 2;

        let spawn = &world.spawns[gamestate.team as usize % 2];
        pos.set(spawn.pos);
        look.yaw = spawn.rot[0];
        look.pitch = spawn.rot[1];
        headyaw.0 = spawn.rot[0];

        client.send_chat_message("Game started!");
    }

    start_game_ev.send(StartGameEvent(game_id));
}

pub fn end_game<T: Resource + DuelsConfig>(
    mut clients: Query<(
        &mut Client,
        &mut PlayerGameState,
        &mut EntityLayerId,
        &mut VisibleChunkLayer,
        &mut VisibleEntityLayers,
        &mut Position,
        &mut Health,
    )>,
    games: Query<(&EntityLayerId, &Entities), Without<PlayerGameState>>,
    mut end_game: EventReader<EndGameEvent>,
    mut commands: Commands,
    mut server_globals: ResMut<ServerGlobals>,
    globals: Res<MapGlobals>,
    config: Res<T>,
) {
    for event in end_game.read() {
        let Ok((game_layer, entities)) = games.get(event.game_id) else {
            continue;
        };
        for entity in entities.0.iter() {
            let Ok((
                mut client,
                mut gamestate,
                mut layer_id,
                mut visible_chunk_layer,
                mut visible_entity_layers,
                mut pos,
                mut health,
            )) = clients.get_mut(*entity)
            else {
                continue;
            };
            layer_id.0 = globals.queue_layer;
            visible_chunk_layer.0 = globals.queue_layer;
            visible_entity_layers.0.clear();
            visible_entity_layers.0.insert(globals.queue_layer);
            pos.set(config.worlds()[0].spawns[0].pos);
            health.0 = 20.0;

            if gamestate.team == event.loser {
                client.send_chat_message("You lost!");
            } else {
                client.send_chat_message("You won!");
                gamestate.wins += 1;
            }

            gamestate.game_id = None;
            gamestate.team = 0;

            server_globals.queue.push(*entity);
        }

        commands.entity(game_layer.0).despawn();
        commands.entity(event.game_id).despawn();
    }
}
