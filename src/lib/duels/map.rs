use valence::prelude::*;
use valence_anvil::AnvilLevel;

use super::*;

#[derive(Bundle)]
pub struct Game {
    pub map: MapIndex,
    pub layer: EntityLayerId,
    pub clients: Entities,
    pub game_start: GameTime,
    pub game_stage: GameStage,
    pub data: GameData,
}

#[derive(Resource)]
pub struct MapGlobals {
    pub map_layers: Vec<Entity>,
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup)
            .add_systems(Update, (
                init_clients,
                start_game.after(init_clients),
                end_game,
            ))
            .add_systems(PostUpdate, check_queue);
    }
}

pub fn setup(
    mut commands: Commands,
    server: Res<Server>,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    config: Res<DuelsConfig>,
) {
    let mut layers: Vec<Entity> = Vec::new();
    for world in config.worlds.iter() {
        let layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);
        let mut level = AnvilLevel::new(world.path.clone(), &biomes);

        for z in world.z_chunks[0]..=world.z_chunks[1] {
            for x in world.x_chunks[0]..=world.x_chunks[1] {
                let pos = ChunkPos::new(x, z);

                level.ignored_chunks.insert(pos);
                level.force_chunk_load(pos);
            }
        }

        layers.push(commands.spawn((layer, level)).id());
    }

    commands.insert_resource(MapGlobals {
        map_layers: layers,
    });
}

pub fn init_clients(
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
    config: Res<DuelsConfig>,
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
        let Ok(layer) = layers.get(globals.map_layers[0]) else {
            continue;
        };

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set(config.worlds[0].spawns[0].pos);
        *game_mode = settings.default_gamemode;
        health.0 = 20.0;
        commands
            .entity(entity)
            .insert((PlayerGameState::default(), CombatState::default()));

        server_globals.queue.push(entity);
    }
}

pub fn check_queue(
    mut start_game: EventWriter<StartGameEvent>,
    server: Res<Server>,
    mut commands: Commands,
    mut globals: ResMut<ServerGlobals>,
) {
    if globals.queue.len() < 2 {
        return;
    }
    fastrand::shuffle(&mut globals.queue);
    while globals.queue.len() > 1 {
        let entitylayer = commands.spawn(EntityLayer::new(&server)).id();

        let game_id = commands
            .spawn(Game {
                map: MapIndex(0),
                layer: EntityLayerId(entitylayer),
                clients: Entities(globals.queue.drain(..2).collect()),
                game_start: GameTime(SystemTime::now()),
                game_stage: GameStage(0),
                data: GameData(HashMap::new()),
            })
            .id();

        start_game.send(StartGameEvent(game_id));
    }
}

pub fn start_game(
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
    mut games: Query<(&mut MapIndex, &EntityLayerId, &Entities), Without<Client>>,
    chunklayers: Query<Entity, With<ChunkLayer>>,
    entitylayers: Query<Entity, With<EntityLayer>>,
    mut start_game: EventReader<StartGameEvent>,
    globals: Res<MapGlobals>,
    config: Res<DuelsConfig>,
) {
    for event in start_game.read() {
        if let Ok((mut map, game_layer, entities)) = games.get_mut(event.0) {
            let map_idx = fastrand::usize(1..globals.map_layers.len());
            map.0 = map_idx;

            for (i, entity) in entities.0.iter().enumerate() {
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
                let Ok(chunklayer) = chunklayers.get(globals.map_layers[map_idx]) else {
                    continue;
                };
                let Ok(entitylayer) = entitylayers.get(game_layer.0) else {
                    continue;
                };

                layer_id.0 = entitylayer;
                visible_chunk_layer.0 = chunklayer;
                visible_entity_layers.0.clear();
                visible_entity_layers.0.insert(entitylayer);

                gamestate.game_id = Some(event.0);
                gamestate.team = i as u8 % 2;

                let spawn = &config.worlds[map_idx].spawns[gamestate.team as usize % 2];
                pos.set(spawn.pos);
                look.yaw = spawn.rot[0];
                look.pitch = spawn.rot[1];
                headyaw.0 = spawn.rot[0];

                client.send_chat_message("Game started!");
            }
        }
    }
}

pub fn end_game(
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
    config: Res<DuelsConfig>,
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
            layer_id.0 = globals.map_layers[0];
            visible_chunk_layer.0 = globals.map_layers[0];
            visible_entity_layers.0.clear();
            visible_entity_layers.0.insert(globals.map_layers[0]);
            pos.set(config.worlds[0].spawns[0].pos);
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