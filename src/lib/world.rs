#![allow(dead_code)]

use valence::inventory::player_inventory::PlayerInventory;
use valence::{interact_block::InteractBlockEvent, inventory::HeldItem, math::IVec3, prelude::*};

#[derive(Event)]
pub struct BlockBreakEvent {
    pub client: Entity,
    pub position: BlockPos,
    pub block: BlockKind,
}

#[derive(Resource)]
struct DiggingPluginResource {
    whitelist: Vec<BlockKind>,
}

pub struct DiggingPlugin {
    pub whitelist: Vec<BlockKind>,
}

impl Plugin for DiggingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DiggingPluginResource {
            whitelist: self.whitelist.clone(),
        })
        .add_event::<BlockBreakEvent>()
        .add_systems(Update, handle_digging_events);
    }
}

fn handle_digging_events(
    mut clients: Query<(&GameMode, &mut Inventory, &VisibleChunkLayer)>,
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<DiggingEvent>,
    mut break_events: EventWriter<BlockBreakEvent>,
    res: Res<DiggingPluginResource>,
) {
    for event in events.read() {
        if let Ok((gamemode, mut inv, layer)) = clients.get_mut(event.client) {
            if *gamemode == GameMode::Adventure
                || *gamemode == GameMode::Spectator
                || event.state != DiggingState::Stop
            {
                continue;
            }
            let Ok(mut chunk_layer) = layers.get_mut(layer.0) else {
                continue;
            };
            let Some(block) = chunk_layer.block(event.position) else {
                continue;
            };
            let block_kind = block.state.to_kind();
            if res.whitelist.contains(&block_kind) {
                let item_kind = block_kind.to_item_kind();
                if let Some(slot) = inv.first_slot_with_item(item_kind, 64) {
                    let count = inv.slot(slot).count + 1;
                    inv.set_slot_amount(slot, count);
                } else if let Some(slot) = inv.first_empty_slot_in(9..45) {
                    inv.set_slot(slot, ItemStack::new(item_kind, 1, None));
                }
                // If it is a bed, break the other half
                if let Some(part) = block.state.get(PropName::Part)
                    && let Some(dir) = block.state.get(PropName::Facing)
                {
                    let dir = match part {
                        PropValue::Head => match dir {
                            PropValue::North => Direction::South,
                            PropValue::East => Direction::West,
                            PropValue::South => Direction::North,
                            PropValue::West => Direction::East,
                            _ => continue,
                        },
                        PropValue::Foot => match dir {
                            PropValue::North => Direction::North,
                            PropValue::East => Direction::East,
                            PropValue::South => Direction::South,
                            PropValue::West => Direction::West,
                            _ => continue,
                        },
                        _ => continue,
                    };
                    let other_pos = event.position.get_in_direction(dir);
                    chunk_layer.set_block(other_pos, BlockState::AIR);
                }
                chunk_layer.set_block(event.position, BlockState::AIR);
                break_events.send(BlockBreakEvent {
                    client: event.client,
                    position: event.position,
                    block: block_kind,
                });
            }
        }
    }
}

#[derive(Event)]
pub struct BlockPlaceEvent {
    pub client: Entity,
    pub position: BlockPos,
    pub block: BlockKind,
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
    max_x: isize,
    min_x: isize,
    max_y: isize,
    min_y: isize,
    max_z: isize,
    min_z: isize,
}

pub struct PlacingPlugin {
    pub max_x: isize,
    pub min_x: isize,
    pub max_y: isize,
    pub min_y: isize,
    pub max_z: isize,
    pub min_z: isize,
}

impl Plugin for PlacingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PlacingPluginResource {
            max_x: self.max_x,
            min_x: self.min_x,
            max_y: self.max_y,
            min_y: self.min_y,
            max_z: self.max_z,
            min_z: self.min_z,
        })
        .add_event::<BlockPlaceEvent>()
        .add_systems(Update, handle_placing_events);
    }
}

// TODO: Nest the loops and if statements so that you only need to call resync_inv once
fn handle_placing_events(
    mut clients: Query<(
        &GameMode,
        &Position,
        &mut Inventory,
        &HeldItem,
        &VisibleChunkLayer,
    )>,
    mut layers: Query<&mut ChunkLayer>,
    mut events: EventReader<InteractBlockEvent>,
    mut placing_events: EventWriter<BlockPlaceEvent>,
    restrictions: Option<Res<PlacingRestrictions>>,
    res: Res<PlacingPluginResource>,
) {
    'outer: for event in events.read() {
        if let Ok((gamemode, pos, mut inv, held_item, layer)) = clients.get_mut(event.client) {
            let block_pos = event.position.get_in_direction(event.face);
            let block_x = block_pos.x as isize;
            let block_y = block_pos.y as isize;
            let block_z = block_pos.z as isize;
            if *gamemode == GameMode::Adventure
                || *gamemode == GameMode::Spectator
                || block_x > res.max_x
                || block_x < res.min_x
                || block_y > res.max_y
                || block_y < res.min_y
                || block_z > res.max_z
                || block_z < res.min_z
            {
                inv.changed |= u64::MAX;
                continue;
            }
            if let Some(restrictions) = &restrictions {
                for area in restrictions.areas.iter() {
                    if block_pos.x >= area.min.x
                        && block_pos.x <= area.max.x
                        && block_pos.y >= area.min.y
                        && block_pos.y <= area.max.y
                        && block_pos.z >= area.min.z
                        && block_pos.z <= area.max.z
                    {
                        inv.changed |= u64::MAX;
                        continue 'outer;
                    }
                }
            }
            let Ok(mut chunk_layer) = layers.get_mut(layer.0) else {
                inv.changed |= u64::MAX;
                continue;
            };
            let slot = match event.hand {
                Hand::Main => held_item.slot(),
                Hand::Off => PlayerInventory::SLOT_OFFHAND,
            };

            if inv.slot(slot).count == 0 {
                inv.changed |= u64::MAX;
                continue;
            }
            let kind = inv.slot(slot).item;
            if let Some(block_kind) = BlockKind::from_item_kind(kind) {
                let diff = pos.0
                    - DVec3::new(
                        block_pos.x as f64 + 0.5,
                        block_pos.y as f64,
                        block_pos.z as f64 + 0.5,
                    );
                if diff.x.abs() > 0.8 || diff.z.abs() > 0.8 || diff.y >= 1.0 || diff.y <= -2.0 {
                    let Some(block) = chunk_layer.block(block_pos) else {
                        inv.changed |= u64::MAX;
                        continue;
                    };
                    if block.state.is_replaceable() {
                        chunk_layer.set_block(block_pos, block_kind.to_state());
                        let count = inv.slot(slot).count - 1;
                        inv.set_slot_amount(slot, count);
                        placing_events.send(BlockPlaceEvent {
                            client: event.client,
                            position: block_pos,
                            block: block_kind,
                        });
                    } else {
                        inv.changed |= u64::MAX;
                    }
                }
            }
        }
    }
}
