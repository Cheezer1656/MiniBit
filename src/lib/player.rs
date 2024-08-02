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
    entity::EntityId,
    event_loop::PacketEvent,
    inventory::HeldItem,
    prelude::*,
    protocol::{
        packets::play::{
            entity_equipment_update_s2c::EquipmentEntry, ClickSlotC2s, EntityEquipmentUpdateS2c,
            UpdateSelectedSlotC2s,
        },
        VarInt, WritePacket,
    },
};

pub struct InvBroadcastPlugin;

impl Plugin for InvBroadcastPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (broadcast_inv_updates, broadcast_slot_updates));
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
