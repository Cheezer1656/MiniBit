use valence::{
    entity::living::LivingFlags,
    event_loop::PacketEvent,
    interact_item::InteractItemEvent,
    inventory::{DropItemStackEvent, PlayerAction},
    prelude::*,
    protocol::packets::play::PlayerActionC2s,
};

pub struct InteractionBroadcastPlugin;

impl Plugin for InteractionBroadcastPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (broadcast_use_item, broadcast_stop_item));
    }
}

fn broadcast_use_item(
    mut clients: Query<&mut LivingFlags, With<Client>>,
    mut events: EventReader<InteractItemEvent>,
) {
    for event in events.read() {
        if let Ok(mut flags) = clients.get_mut(event.client) {
            flags.set_using_item(true);
        }
    }
}

fn broadcast_stop_item(
    mut clients: Query<&mut LivingFlags, With<Client>>,
    mut packets: EventReader<PacketEvent>,
) {
    for packet in packets.read() {
        if let Some(pkt) = packet.decode::<PlayerActionC2s>()
            && pkt.action == PlayerAction::ReleaseUseItem
            && let Ok(mut flags) = clients.get_mut(packet.client)
        {
            flags.set_using_item(false);
        }
    }
}

pub struct DisableDropPlugin;

impl Plugin for DisableDropPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, handle_drop);
    }
}

fn handle_drop(
    mut clients: Query<(&mut Inventory, &mut CursorItem)>,
    mut drop_events: EventReader<DropItemStackEvent>,
) {
    for event in drop_events.read() {
        if let Ok((mut inv, mut cursor_item)) = clients.get_mut(event.client) {
            if let Some(slot) = event.from_slot {
                if inv.slot(slot).item == event.stack.item {
                    let count = inv.slot(slot).count;
                    inv.set_slot(
                        slot,
                        event.stack.clone().with_count(count + event.stack.count),
                    );
                } else {
                    inv.set_slot(slot, event.stack.clone());
                }
            } else {
                cursor_item.0 = event.stack.clone();
            }
            inv.changed = u64::MAX;
        }
    }
}
