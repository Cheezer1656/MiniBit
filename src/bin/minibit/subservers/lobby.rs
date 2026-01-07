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

mod commands;

use std::{
    marker::PhantomData,
    time::{Duration, SystemTime},
};
use std::path::PathBuf;
use minibit_lib::{config::{ConfigLoaderPlugin, WorldValue}, player::*, scopes::ScopePlugin};
use serde::Deserialize;
use valence::{
    entity::{living::Health, player::{PlayerEntityBundle, PlayerModelParts}}, event_loop::PacketEvent, inventory::{ClickSlotEvent, HeldItem}, message::{ChatMessageEvent, SendMessage}, nbt::{compound, List}, player_list::{DisplayName, Listed, PlayerListEntryBundle}, prelude::*, protocol::{packets::play::PlayerInteractItemC2s, sound::SoundCategory, Sound}
};
use valence_anvil::AnvilLevel;
use minibit_lib::config::DataPath;

#[derive(Deserialize, Clone)]
enum ActionType {
    Message,
    Warp,
    None,
}

#[derive(Event)]
struct ActionEvent {
    entity: Entity,
    action: ActionType,
    args: Vec<String>,
}

#[derive(Component, Clone)]
struct NpcAction {
    command: ActionType,
    args: Vec<String>,
}

#[derive(Deserialize)]
struct NpcValue {
    name: String,
    pos: [f64; 3],
    yaw: f32,
    pitch: f32,
    skin: String,
    signature: String,
    command: ActionType,
    args: Vec<String>,
}

#[derive(Deserialize)]
struct ParkourConfig {
    name: String,
    start: [f64; 3],
    end: [f64; 3],
}

#[derive(Resource, Deserialize)]
struct LobbyConfig {
    world: WorldValue,
    npcs: Vec<NpcValue>,
    parkour: Vec<ParkourConfig>,
}

#[derive(Resource)]
struct ServerGlobals {
    navigator_gui: Option<Entity>,
}

#[derive(Component)]
struct ParkourStatus {
    name: String,
    start: SystemTime,
    end: DVec3,
}

pub fn main(path: PathBuf) {
    App::new()
        .add_plugins(ConfigLoaderPlugin::<LobbyConfig> {
            path,
            phantom: PhantomData,
        })
        .add_plugins(DefaultPlugins)
        .add_plugins((ScopePlugin, commands::CommandPlugin, InteractionBroadcastPlugin))
        .insert_resource(ServerGlobals {
            navigator_gui: None,
        })
        .add_event::<ActionEvent>()
        .add_systems(Startup, setup)
        .add_systems(EventLoopUpdate, (item_interactions, handle_slot_click))
        .add_systems(
            Update,
            (
                despawn_disconnected_clients,
                init_clients,
                manage_players,
                entity_interactions,
                chat_message,
                start_parkour,
                manage_parkour,
                execute_action,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    server: Res<Server>,
    config: Res<LobbyConfig>,
    data_path: Res<DataPath>,
    mut globals: ResMut<ServerGlobals>,
) {
    let layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);
    let mut level = AnvilLevel::new(data_path.0.join(&config.world.path), &biomes);

    for z in config.world.z_chunks[0]..=config.world.z_chunks[1] {
        for x in config.world.x_chunks[0]..=config.world.x_chunks[1] {
            let pos = ChunkPos::new(x, z);

            level.ignored_chunks.insert(pos);
            level.force_chunk_load(pos);
        }
    }

    let layer_id = commands.spawn((layer, level)).id();

    for npc in &config.npcs {
        let npc_id = UniqueId::default();

        commands.spawn(PlayerEntityBundle {
            layer: EntityLayerId(layer_id),
            uuid: npc_id,
            position: Position::new(npc.pos),
            look: Look::new(npc.yaw, npc.pitch),
            head_yaw: HeadYaw(npc.yaw),
            player_player_model_parts: PlayerModelParts(126),
            ..PlayerEntityBundle::default()
        }).insert(NpcAction {
            command: npc.command.clone(),
            args: npc.args.clone(),
        });

        let mut props = Properties::default();
        props.set_skin(npc.skin.clone(), npc.signature.clone());

        commands.spawn(PlayerListEntryBundle {
            uuid: npc_id,
            username: Username(npc.name.clone()),
            display_name: DisplayName(Some(npc.name.clone().color(Color::RED))),
            listed: Listed(false),
            properties: props,
            ..Default::default()
        });
    }

    let mut navigator_inv = Inventory::with_title(InventoryKind::Generic9x6, "Server Navigator");
    navigator_inv.readonly = true;
    navigator_inv.set_slot(4, ItemStack::new(ItemKind::Compass, 1, Some(compound! {
        "display" => compound! {
            "Name" => "{\"text\":\"Games\",\"italic\":false}"
        },
    })));

    for i in 45..54 {
        navigator_inv.set_slot(i as u16, ItemStack::new(ItemKind::GrayStainedGlassPane, 1, None));
    }
    for i in (0..4).chain(5..9) {
        navigator_inv.set_slot(i as u16, ItemStack::new(ItemKind::GrayStainedGlassPane, 1, None));
    }

    for (i, npc) in config.npcs.iter().enumerate() {
        if i > 20 {
            break;
        }
        let row = i / 7;
        let col = i % 7;
        navigator_inv.set_slot(
            (row * 9 + col + 19) as u16,
            ItemStack::new(
                ItemKind::PlayerHead,
                1,
                Some(compound! {
                    "display" => compound! {
                        "Name" => format!("{{\"text\":\"{}\",\"italic\":false}}", npc.name)
                    },
                    "SkullOwner" => compound! {
                        "Name" => "Notch",
                        "Properties" => compound! {
                            "textures" => List::from(vec![compound! {
                                "Value" => &npc.skin,
                                "Signature" => &npc.signature
                            }])
                        }
                    }
                }),
            ),
        );
    }
    globals.navigator_gui = Some(commands.spawn(navigator_inv).id());
}

fn init_clients(
    mut clients: Query<
        (
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut Look,
            &mut HeadYaw,
            &mut GameMode,
            &mut Health,
            &mut Inventory,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, With<ChunkLayer>>,
    config: Res<LobbyConfig>,
) {
    for (
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut look,
        mut head_yaw,
        mut game_mode,
        mut health,
        mut inv,
    ) in &mut clients
    {
        let layer = layers.single();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set(config.world.spawns[0].pos);
        look.yaw = config.world.spawns[0].rot[0];
        look.pitch = config.world.spawns[0].rot[1];
        head_yaw.0 = config.world.spawns[0].rot[0];
        *game_mode = GameMode::Adventure;
        health.0 = 20.0;

        inv.set_slot(
            36,
            ItemStack::new(
                ItemKind::Compass,
                1,
                Some(compound! {
                    "display" => compound! {
                        "Name" => "{\"text\":\"Navigator\",\"italic\":false}"
                    },
                }),
            ),
        );

        inv.readonly = true;
    }
}

fn manage_players(
    mut clients: Query<(&mut Client, &mut Position, &HeadYaw), With<Client>>,
    mut layers: Query<&mut ChunkLayer>,
    config: Res<LobbyConfig>,
) {
    let layer = layers.single_mut();
    for (mut client, mut pos, yaw) in clients.iter_mut() {
        if pos.0.y < 0.0 {
            pos.set(config.world.spawns[0].pos);
        }
        let Some(block) = layer.block(BlockPos::new(
            pos.0.x.floor() as i32,
            pos.0.y.ceil() as i32 - 1,
            pos.0.z.floor() as i32,
        )) else {
            continue;
        };
        if block.state == BlockState::SLIME_BLOCK {
            client.play_sound(
                Sound::EntityFireworkRocketLaunch,
                SoundCategory::Master,
                pos.0,
                1.0,
                1.0,
            );
            let yaw = yaw.0.to_radians();
            client.set_velocity(Vec3::new(-yaw.sin() * 65.0, 30.0, yaw.cos() * 65.0));
        }
    }
}

fn entity_interactions(
    actions: Query<&NpcAction>,
    mut events: EventReader<InteractEntityEvent>,
    mut action_event: EventWriter<ActionEvent>,
) {
    for event in events.read() {
        match event.interact {
            EntityInteraction::Attack => {}
            EntityInteraction::Interact(hand) => {
                if hand != Hand::Main {
                    continue;
                }
            }
            _ => continue,
        }
        let Ok(action) = actions.get(event.entity) else {
            continue;
        };

        action_event.send(ActionEvent {
            entity: event.client,
            action: action.command.clone(),
            args: action.args.clone(),
        });
    }
}

fn item_interactions(
    mut clients: Query<(Entity, &mut Inventory, &HeldItem), With<Client>>,
    mut packets: EventReader<PacketEvent>,
    mut commands: Commands,
    globals: Res<ServerGlobals>,
) {
    for packet in packets.read() {
        if let Some(_pkt) = packet.decode::<PlayerInteractItemC2s>()
            && let Ok((entity, mut inv, item)) = clients.get_mut(packet.client)
        {
            match inv.slot(item.slot()).item {
                ItemKind::Compass => {
                    commands
                        .entity(entity)
                        .insert(OpenInventory::new(globals.navigator_gui.unwrap()));
                }
                ItemKind::Barrier => {
                    commands.entity(entity).remove::<ParkourStatus>();
                    inv.set_slot(item.slot(), ItemStack::EMPTY);
                }
                _ => {}
            }
        }
    }
}

fn handle_slot_click(
    clients: Query<&OpenInventory, With<Client>>,
    mut action_event: EventWriter<ActionEvent>,
    mut click_slot: EventReader<ClickSlotEvent>,
    config: Res<LobbyConfig>,
) {
    for event in click_slot.read() {
        if let Ok(_open_inv) = clients.get(event.client) && event.window_id != 0 && event.slot_id >= 19 {
            let offset_slot = event.slot_id as usize - 19;
            let row = offset_slot / 9;
            let col = offset_slot % 9;
            let npc = row * 7 + col;

            if npc < config.npcs.len() {
                action_event.send(ActionEvent {
                    entity: event.client,
                    action: config.npcs[npc].command.clone(),
                    args: config.npcs[npc].args.clone(),
                });
            }
        }
    }
}

fn chat_message(
    usernames: Query<&Username>,
    mut clients: Query<&mut Client>,
    mut events: EventReader<ChatMessageEvent>,
) {
    for event in events.read() {
        let Ok(username) = usernames.get(event.client) else {
            continue;
        };
        for mut client in clients.iter_mut() {
            client.send_chat_message(
                (String::new() + &username.0 + &String::from(": ") + &event.message)
                    .color(Color::GRAY),
            );
        }
    }
}

fn start_parkour(
    mut query: Query<(Entity, &mut Client, &mut Inventory, &Position), Without<ParkourStatus>>,
    mut commands: Commands,
    config: Res<LobbyConfig>,
) {
    for (entity, mut client, mut inv, pos) in query.iter_mut() {
        for parkour in &config.parkour {
            if pos.0.floor() == parkour.start.into() {
                client.send_chat_message(
                    (String::new() + &parkour.name + " started!")
                        .into_text()
                        .bold()
                        .color(Color::GREEN),
                );
                commands.entity(entity).insert(ParkourStatus {
                    name: parkour.name.clone(),
                    start: SystemTime::now(),
                    end: parkour.end.into(),
                });
                inv.set_slot(
                    44,
                    ItemStack::new(
                        ItemKind::Barrier,
                        1,
                        Some(compound! {
                            "display" => compound! {
                                "Name" => "{\"text\":\"Cancel Parkour\",\"italic\":false}"
                            },
                        }),
                    ),
                );
            }
        }
    }
}

fn manage_parkour(
    mut query: Query<(Entity, &mut Client, &ParkourStatus, &Position), With<ParkourStatus>>,
    mut commands: Commands,
) {
    for (entity, mut client, status, pos) in query.iter_mut() {
        let time = &format!(
            "{:.1}",
            &status
                .start
                .elapsed()
                .unwrap_or(Duration::new(0, 0))
                .as_secs_f32()
        );
        client.set_action_bar(String::new() + &status.name + " - " + time + "s");
        if pos.0.floor() == status.end {
            client.send_chat_message(
                (String::new() + &status.name + " completed in " + time + " seconds!")
                    .into_text()
                    .bold()
                    .color(Color::GREEN),
            );
            commands.entity(entity).remove::<ParkourStatus>();
        }
    }
}

fn execute_action(
    mut events: EventReader<ActionEvent>,
    mut clients: Query<(&mut Client, &Username)>,
) {
    for event in events.read() {
        if let Ok((mut client, username)) = clients.get_mut(event.entity) {
            match event.action {
                ActionType::Message => {
                    for arg in &event.args {
                        client.send_chat_message(arg.clone().into_text().bold());
                    }
                }
                ActionType::Warp => {
                    let mut payload: Vec<u8> = Vec::new();
                    payload.extend_from_slice("1".as_bytes());
                    payload.push(0);
                    payload.extend_from_slice(username.0.to_string().as_bytes());
                    payload.push(0);
                    payload.extend_from_slice(event.args[0].as_bytes());
                    client.send_custom_payload(ident!("minibit:main"), &payload);
                }
                ActionType::None => {}
            }
        }
    }
}
