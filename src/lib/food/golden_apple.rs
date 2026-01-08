use valence::prelude::*;
use valence::client::Client;
use valence::interact_item::InteractItemEvent;
use valence::inventory::{HeldItem, Inventory, PlayerAction};
use valence::{Hand, ItemKind, Server};
use valence::entity::EntityStatus;
use valence::entity::living::{Absorption, Health};
use valence::event_loop::PacketEvent;
use valence::protocol::packets::play::PlayerActionC2s;

pub struct GoldenApplePlugin;

impl Plugin for GoldenApplePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            init_clients,
            set_use_tick,
            eat_gapple,
            cancel_gapple,
        ));
    }
}

#[derive(Component)]
struct EatingStartTick(pub i64);

fn init_clients(clients: Query<Entity, Added<Client>>, mut commands: Commands) {
    for entity in clients.iter() {
        commands.entity(entity).insert(EatingStartTick(i64::MAX));
    }
}

fn set_use_tick(
    mut clients: Query<(&Inventory, &HeldItem, &mut EatingStartTick), With<Client>>,
    mut events: EventReader<InteractItemEvent>,
    server: Res<Server>,
) {
    for event in events.read() {
        if let Ok((inv, held_item, mut eat_tick)) = clients.get_mut(event.client)
            && event.hand == Hand::Main
            && inv.slot(held_item.slot()).item == ItemKind::GoldenApple
        {
            eat_tick.0 = server.current_tick();
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
        let slot = held_item.slot();
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
            inv.set_slot_amount(held_item.slot(), count - 1);
        }
    }
}

fn cancel_gapple(
    mut clients: Query<(&Inventory, &HeldItem, &mut EatingStartTick), With<Client>>,
    mut packets: EventReader<PacketEvent>,
) {
    for packet in packets.read() {
        if let Some(pkt) = packet.decode::<PlayerActionC2s>()
            && pkt.action == PlayerAction::ReleaseUseItem
            && let Ok((inv, held_item, mut eat_tick)) = clients.get_mut(packet.client)
            && inv.slot(held_item.slot()).item == ItemKind::GoldenApple
        {
            eat_tick.0 = i64::MAX;
        }
    }
}
