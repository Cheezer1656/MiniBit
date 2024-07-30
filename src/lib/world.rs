use valence::prelude::*;

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