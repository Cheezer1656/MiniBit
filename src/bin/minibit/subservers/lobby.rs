#![allow(clippy::type_complexity)]

mod commands;

use crate::ServerConfig;
use minibit_lib::config::DataPath;
use minibit_lib::scoreboard::{ScoreboardMode, ScoreboardPlugin};
use minibit_lib::{config::{ConfigLoaderPlugin, WorldValue}, player::*, scopes::ScopePlugin};
use serde::Deserialize;
use std::{
    marker::PhantomData,
    time::{Duration, SystemTime},
};
use chunkedge::anvil::AnvilLevel;
use chunkedge::item::{ItemComponent, ProfileProperty, ResolvableProfile};
use chunkedge::protocol::packets::play::UseItemC2s;
use chunkedge::protocol::IntoTextComponent;
use chunkedge::{
    entity::{living::Health, player::PlayerModelParts}, inventory::HeldItem, message::SendMessage, player_list::{DisplayName, Listed, PlayerListEntryBundle}, prelude::*, protocol::{sound::SoundCategory, Sound}
};
use chunkedge::entity::player::PlayerEntity;
use chunkedge::event_loop::PacketMessage;
use chunkedge::inventory::ClickSlotMessage;
use chunkedge::message::ChatReceivedMessage;

#[derive(Deserialize, Clone)]
enum ActionType {
    Message,
    Warp,
    None,
}

#[derive(Message)]
struct ActionMessage {
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

pub fn main(config: ServerConfig) {
    App::new()
        .add_plugins(ConfigLoaderPlugin::<LobbyConfig> {
            path: config.path,
            network_config: config.network,
            phantom: PhantomData,
        })
        .add_plugins(DefaultPlugins)
        .add_plugins((ScopePlugin, commands::CommandPlugin, ScoreboardPlugin {
            name: "MINIBIT",
            text: vec!["Welcome to MiniBit!"],
            mode: ScoreboardMode::ServerWide,
        }, InteractionBroadcastPlugin))
        .insert_resource(ServerGlobals {
            navigator_gui: None,
        })
        .add_message::<ActionMessage>()
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

        commands.spawn((
            PlayerEntity,
            EntityLayerId(layer_id),
            npc_id,
            Position::new(npc.pos),
            Look::new(npc.yaw, npc.pitch),
            HeadYaw(npc.yaw),
            PlayerModelParts(126),
        )).insert(NpcAction {
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
    navigator_inv.set_slot(4, ItemStack::new(ItemKind::Compass, 1).with_components(vec![
        ItemComponent::ItemName("Games".into_text_component()),
    ]));

    for i in 45..54 {
        navigator_inv.set_slot(i as u16, ItemStack::new(ItemKind::GrayStainedGlassPane, 1));
    }
    for i in (0..4).chain(5..9) {
        navigator_inv.set_slot(i as u16, ItemStack::new(ItemKind::GrayStainedGlassPane, 1));
    }

    for (i, npc) in config.npcs.iter().enumerate() {
        if i > 20 {
            break;
        }
        let row = i / 7;
        let col = i % 7;
        navigator_inv.set_slot(
            (row * 9 + col + 19) as u16,
            ItemStack::new(ItemKind::PlayerHead, 1).with_components(vec![
                ItemComponent::ItemName(npc.name.clone().into_text_component()),
                ItemComponent::Profile(ResolvableProfile {
                    name: Some(npc.name.clone().replace(" ", "")),
                    id: None,
                    properties: vec![
                        ProfileProperty {
                            name: String::from("textures"),
                            value: npc.skin.clone(),
                            signature: Some(npc.signature.clone()),
                        }
                    ],
                })
            ]),
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
        let layer = layers.single().unwrap();

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
            ItemStack::new(ItemKind::Compass, 1).with_components(vec![
                ItemComponent::ItemName("Navigator".into_text_component()),
            ]),
        );

        inv.readonly = true;
    }
}

fn manage_players(
    mut clients: Query<(&mut Client, &mut Position, &HeadYaw), With<Client>>,
    mut layers: Query<&mut ChunkLayer>,
    config: Res<LobbyConfig>,
) {
    let layer = layers.single_mut().unwrap();
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
    mut messages: MessageReader<InteractEntityMessage>,
    mut action_event: MessageWriter<ActionMessage>,
) {
    for message in messages.read() {
        match message.interact {
            EntityInteraction::Attack => {}
            EntityInteraction::Interact(hand) => {
                if hand != Hand::Main {
                    continue;
                }
            }
            _ => continue,
        }
        let Ok(action) = actions.get(message.entity) else {
            continue;
        };

        action_event.write(ActionMessage {
            entity: message.client,
            action: action.command.clone(),
            args: action.args.clone(),
        });
    }
}

fn item_interactions(
    mut clients: Query<(Entity, &mut Inventory, &HeldItem), With<Client>>,
    mut packets: MessageReader<PacketMessage>,
    mut commands: Commands,
    globals: Res<ServerGlobals>,
) {
    for packet in packets.read() {
        if let Some(_pkt) = packet.decode::<UseItemC2s>()
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
    mut action_event: MessageWriter<ActionMessage>,
    mut click_slot: MessageReader<ClickSlotMessage>,
    config: Res<LobbyConfig>,
) {
    for message in click_slot.read() {
        if let Ok(_open_inv) = clients.get(message.client) && message.window_id.0 != 0 && message.slot_id >= 19 {
            let offset_slot = message.slot_id as usize - 19;
            let row = offset_slot / 9;
            let col = offset_slot % 9;
            let npc = row * 7 + col;

            if npc < config.npcs.len() {
                action_event.write(ActionMessage {
                    entity: message.client,
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
    mut messages: MessageReader<ChatReceivedMessage>,
) {
    for message in messages.read() {
        let Ok(username) = usernames.get(message.client) else {
            continue;
        };
        for mut client in clients.iter_mut() {
            client.send_chat_message(
                (String::new() + &username.0 + &String::from(": ") + &message.message)
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
                    ItemStack::new(ItemKind::Barrier, 1).with_components(vec![
                        ItemComponent::ItemName("Cancel Parkour".into_text_component()),
                    ]),
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
            status
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
    mut messages: MessageReader<ActionMessage>,
    mut clients: Query<(&mut Client, &Username)>,
) {
    for message in messages.read() {
        if let Ok((mut client, username)) = clients.get_mut(message.entity) {
            match message.action {
                ActionType::Message => {
                    for arg in &message.args {
                        client.send_chat_message(arg.clone().into_text().bold());
                    }
                }
                ActionType::Warp => {
                    let mut payload: Vec<u8> = Vec::new();
                    payload.extend_from_slice("1".as_bytes());
                    payload.push(0);
                    payload.extend_from_slice(username.0.to_string().as_bytes());
                    payload.push(0);
                    payload.extend_from_slice(message.args[0].as_bytes());
                    client.send_custom_payload(ident!("minibit:main"), &payload);
                }
                ActionType::None => {}
            }
        }
    }
}
