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

use valence::{
    entity::{living::LivingFlags, EntityId}, event_loop::PacketEvent, interact_item::InteractItemEvent, inventory::{DropItemStackEvent, HeldItem, PlayerAction}, prelude::*, protocol::{
        packets::play::{
            entity_equipment_update_s2c::EquipmentEntry, ClickSlotC2s, EntityEquipmentUpdateS2c, PlayerActionC2s, UpdateSelectedSlotC2s
        },
        VarInt, WritePacket,
    }
};

pub struct InvBroadcastPlugin;

impl Plugin for InvBroadcastPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            broadcast_inv_updates,
            broadcast_slot_updates,
            broadcast_use_item,
            broadcast_stop_item,
        ));
    }
}

fn broadcast_inv_updates(
    mut clients: Query<(&mut Client, &EntityLayerId)>,
    mut clients2: Query<(&EntityId, &EntityLayerId, &mut Inventory, &HeldItem), With<Client>>,
) {
    for (entity_id, layer_id, inv, held_item) in clients2.iter_mut() {
        if inv.is_changed() {
            let update_packet = EntityEquipmentUpdateS2c {
                entity_id: VarInt(entity_id.get()),
                equipment: vec![
                    EquipmentEntry {
                        slot: 0,
                        item: inv.slot(held_item.slot()).clone(),
                    },
                    EquipmentEntry {
                        slot: 2,
                        item: inv.slot(8).clone(),
                    },
                    EquipmentEntry {
                        slot: 3,
                        item: inv.slot(7).clone(),
                    },
                    EquipmentEntry {
                        slot: 4,
                        item: inv.slot(6).clone(),
                    },
                    EquipmentEntry {
                        slot: 5,
                        item: inv.slot(5).clone(),
                    },
                ],
            };
            for (mut client, layer) in clients.iter_mut() {
                if layer == layer_id {
                    client.write_packet(&update_packet);
                }
            }
        }
    }
}

fn broadcast_slot_updates(
    mut clients: Query<(&mut Client, &EntityLayerId)>,
    clients2: Query<(&EntityId, &EntityLayerId, &Inventory, &HeldItem), With<Client>>,
    mut packets: EventReader<PacketEvent>,
) {
    for pkt in packets.read() {
        if let Some(packet) = pkt.decode::<ClickSlotC2s>() {
            if packet.window_id != 0 || !(packet.slot_idx >= 36 && packet.slot_idx <= 44) {
                continue;
            }
        } else if let Some(_) = pkt.decode::<UpdateSelectedSlotC2s>() {
        } else {
            continue;
        }

        if let Ok((entity_id, layer_id, inv, held_item)) = clients2.get(pkt.client) {
            let update_packet = EntityEquipmentUpdateS2c {
                entity_id: VarInt(entity_id.get()),
                equipment: vec![EquipmentEntry {
                    slot: 0,
                    item: inv.slot(held_item.slot()).clone(),
                }],
            };
            for (mut client, layer) in clients.iter_mut() {
                if layer == layer_id {
                    client.write_packet(&update_packet);
                }
            }
        }
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
        if let Some(pkt) = packet.decode::<PlayerActionC2s>() {
            if pkt.action == PlayerAction::ReleaseUseItem {
                if let Ok(mut flags) = clients.get_mut(packet.client) {
                    flags.set_using_item(false);
                }
            }
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
                    inv.set_slot(slot, event.stack.clone().with_count(count + event.stack.count));
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