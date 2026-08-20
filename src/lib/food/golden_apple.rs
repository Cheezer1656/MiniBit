#![allow(clippy::type_complexity)]

use chunkedge::client::Client;
use chunkedge::entity::EntityStatus;
use chunkedge::entity::living::{Absorption, Health};
use chunkedge::event_loop::PacketMessage;
use chunkedge::interact_item::InteractItemMessage;
use chunkedge::inventory::player_inventory::PlayerInventory;
use chunkedge::inventory::{HeldItem, Inventory, PlayerAction};
use chunkedge::prelude::*;
use chunkedge::protocol::packets::play::PlayerActionC2s;
use chunkedge::{Hand, ItemKind, Server};

pub struct GoldenApplePlugin;

impl Plugin for GoldenApplePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (init_clients, set_use_tick, eat_gapple, cancel_gapple),
        );
    }
}

#[derive(Component)]
struct EatingStartTick(pub i64, pub Hand);

fn init_clients(clients: Query<Entity, Added<Client>>, mut commands: Commands) {
    for entity in clients.iter() {
        commands
            .entity(entity)
            .insert(EatingStartTick(i64::MAX, Hand::Main));
    }
}

fn set_use_tick(
    mut clients: Query<(&Inventory, &HeldItem, &mut EatingStartTick), With<Client>>,
    mut messages: MessageReader<InteractItemMessage>,
    server: Res<Server>,
) {
    for message in messages.read() {
        if let Ok((inv, held_item, mut eat_tick)) = clients.get_mut(message.client)
            && inv
                .slot(match message.hand {
                    Hand::Main => held_item.slot(),
                    Hand::Off => PlayerInventory::SLOT_OFFHAND,
                })
                .item
                == ItemKind::GoldenApple
        {
            eat_tick.0 = server.current_tick();
            eat_tick.1 = message.hand;
        }
    }
}

fn eat_gapple(
    mut clients: Query<
        (
            &mut Client,
            &mut Health,
            &mut Absorption,
            &mut Inventory,
            &HeldItem,
            &mut EatingStartTick,
        ),
        With<Client>,
    >,
    server: Res<Server>,
) {
    for (mut client, mut health, mut absorption, mut inv, held_item, mut eat_tick) in
        clients.iter_mut()
    {
        let slot = match eat_tick.1 {
            Hand::Main => held_item.slot(),
            Hand::Off => PlayerInventory::SLOT_OFFHAND,
        };

        if inv.slot(slot).item != ItemKind::GoldenApple {
            eat_tick.0 = i64::MAX;
            continue;
        }
        if server.current_tick() - eat_tick.0 > 32 {
            eat_tick.0 = i64::MAX;
            client.trigger_status(EntityStatus::ConsumeItem);
            health.0 = 20.0;
            absorption.0 = 4.0;
            let count = inv.slot(slot).count;
            inv.set_slot_amount(slot, count - 1);
        }
    }
}

fn cancel_gapple(
    mut clients: Query<(&Inventory, &HeldItem, &mut EatingStartTick), With<Client>>,
    mut packets: MessageReader<PacketMessage>,
) {
    for packet in packets.read() {
        if let Some(pkt) = packet.decode::<PlayerActionC2s>()
            && pkt.action == PlayerAction::ReleaseUseItem
            && let Ok((inv, held_item, mut eat_tick)) = clients.get_mut(packet.client)
            && inv
                .slot(match eat_tick.1 {
                    Hand::Main => held_item.slot(),
                    Hand::Off => PlayerInventory::SLOT_OFFHAND,
                })
                .item
                == ItemKind::GoldenApple
        {
            eat_tick.0 = i64::MAX;
        }
    }
}
