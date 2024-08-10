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

use valence::{interact_block::InteractBlockEvent, inventory::HeldItem, math::IVec3, prelude::*};

#[derive(Resource)]
struct DiggingPluginResource {
    whitelist: Vec<BlockKind>,
}

pub struct DiggingPlugin {
    pub whitelist: Vec<BlockKind>,
}

impl Plugin for DiggingPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(DiggingPluginResource {
                whitelist: self.whitelist.clone(),
            })
            .add_systems(Update, handle_digging_events);
    }
}

fn handle_digging_events(
    mut clients: Query<(&GameMode, &mut Inventory, &VisibleChunkLayer)>,
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<DiggingEvent>,
    res: Res<DiggingPluginResource>,
) {
    for event in events.read() {
        if let Ok((gamemode, mut inv, layer)) = clients.get_mut(event.client) {
            if *gamemode == GameMode::Adventure || *gamemode == GameMode::Spectator || event.state != DiggingState::Stop {
                continue;
            }
            let Ok(mut chunk_layer) = layers.get_mut(layer.0) else {
                continue;
            };
            let Some(block) = chunk_layer.block(event.position) else {
                continue;
            };
            let kind = block.state.to_kind();
            if res.whitelist.contains(&kind) {
                let kind = kind.to_item_kind();
                if let Some(slot) = inv.first_slot_with_item(kind, 64) {
                    let count = inv.slot(slot).count + 1;
                    inv.set_slot_amount(slot, count);
                } else if let Some(slot) = inv.first_empty_slot_in(9..45) {
                    inv.set_slot(slot, ItemStack::new(kind, 1, None));
                }
                chunk_layer.set_block(event.position, BlockState::AIR);
            }
        }
    }
}

pub struct BlockArea {
    pub min: IVec3,
    pub max: IVec3,
}

#[derive(Resource)]
pub struct PlacingRestrictions {
    pub areas: Vec<BlockArea>,
}

#[derive(Resource)]
struct PlacingPluginResource {
    build_limit: isize,
}

pub struct PlacingPlugin {
    pub build_limit: isize,
}

impl Plugin for PlacingPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(PlacingPluginResource {
                build_limit: self.build_limit,
            })
            .add_systems(Update, handle_placing_events);
    }
}

fn handle_placing_events(
    mut clients: Query<(&GameMode, &Position, &mut Inventory, &HeldItem, &VisibleChunkLayer)>,
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<InteractBlockEvent>,
    restrictions: Option<Res<PlacingRestrictions>>,
    res: Res<PlacingPluginResource>,
) {
    'outer: for event in events.read() {
        if let Ok((gamemode, pos, mut inv, held_item, layer)) = clients.get_mut(event.client) {
            let block_pos = event.position.get_in_direction(event.face);
            if *gamemode == GameMode::Adventure || *gamemode == GameMode::Spectator || event.hand != Hand::Main || block_pos.y as isize > res.build_limit {
                continue;
            }
            if let Some(restrictions) = &restrictions {
                for area in restrictions.areas.iter() {
                    if block_pos.x >= area.min.x && block_pos.x <= area.max.x && block_pos.y >= area.min.y && block_pos.y <= area.max.y && block_pos.z >= area.min.z && block_pos.z <= area.max.z {
                        continue 'outer;
                    }
                }
            }
            let Ok(mut chunk_layer) = layers.get_mut(layer.0) else {
                continue;
            };
            let slot = held_item.slot();
            if inv.slot(slot).count == 0 {
                continue;
            }
            let kind = inv.slot(slot).item;
            if let Some(block_kind) = BlockKind::from_item_kind(kind) {
                let diff = pos.0 - DVec3::new(block_pos.x as f64 + 0.5, block_pos.y as f64, block_pos.z as f64 + 0.5);
                if diff.x.abs() > 0.8 || diff.z.abs() > 0.8 || diff.y >= 1.0 || diff.y <= -2.0 {
                    let Some(block) = chunk_layer.block(block_pos) else {
                        continue;
                    };
                    if block.state.is_replaceable() {
                        chunk_layer.set_block(block_pos, block_kind.to_state());
                        let count = inv.slot(slot).count - 1;
                        inv.set_slot_amount(slot, count);
                    }
                }
            }
        }
    }
}
