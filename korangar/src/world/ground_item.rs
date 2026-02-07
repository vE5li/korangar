use std::sync::Arc;

use cgmath::{Point3, Vector3};
use korangar_interface::element::StateElement;
use ragnarok_packets::{ClientTick, Direction, EntityId, ItemId, TilePosition};
use rust_state::RustState;

use crate::FadeDirection;
use crate::graphics::EntityInstruction;
use crate::loaders::GAT_TILE_SIZE;
use crate::world::{AnimationData, AnimationState, Camera, EntityType, FadeState, ItemResource, ItemResourceKey, Library, Map};

pub const ITEM_SPRITE_PREFIX: &str = "아이템\\";

const FADE_IN_DURATION_MS: u32 = 100;
const FADE_OUT_DURATION_MS: u32 = 250;

#[derive(Clone, RustState, StateElement)]
pub struct GroundItem {
    pub entity_id: EntityId,
    pub item_id: ItemId,
    pub quantity: u16,
    pub is_identified: bool,
    pub tile_position: TilePosition,
    pub world_position: Point3<f32>,
    #[hidden_element]
    animation_state: AnimationState,
    #[hidden_element]
    animation_data: Option<Arc<AnimationData>>,
    #[hidden_element]
    fade_state: FadeState,
}

impl GroundItem {
    pub fn new(
        map: &Map,
        item_id: ItemId,
        entity_id: EntityId,
        is_identified: bool,
        quantity: u16,
        tile_position: TilePosition,
        x_offset: u8,
        y_offset: u8,
        client_tick: ClientTick,
    ) -> Option<Self> {
        const SUBTILE_DIVISOR: f32 = 12.0;

        let mut world_position = map.get_world_position(tile_position)?;
        let offset_x = (x_offset as f32 / SUBTILE_DIVISOR - 0.5) * GAT_TILE_SIZE;
        let offset_z = (y_offset as f32 / SUBTILE_DIVISOR - 0.5) * GAT_TILE_SIZE;
        world_position += Vector3::new(offset_x, 0.0, offset_z);

        Some(Self {
            entity_id,
            item_id,
            quantity,
            is_identified,
            tile_position,
            world_position,
            animation_state: AnimationState::new(EntityType::Npc, client_tick),
            animation_data: None,
            fade_state: FadeState::new(FADE_IN_DURATION_MS, client_tick),
        })
    }

    pub fn update(&mut self, client_tick: ClientTick) {
        self.animation_state.update(client_tick);

        if self.fade_state.is_fading() && self.fade_state.is_done_fading_in(client_tick) {
            self.fade_state = FadeState::Opaque;
        }
    }

    pub fn fade_out(&mut self, client_tick: ClientTick) {
        let fade_state = &mut self.fade_state;

        let current_alpha = fade_state.calculate_alpha(client_tick);
        *fade_state = FadeState::from_alpha(current_alpha, FadeDirection::Out, client_tick, FADE_OUT_DURATION_MS);
    }

    pub fn should_be_removed(&self, client_tick: ClientTick) -> bool {
        self.fade_state.is_done_fading_out(client_tick)
    }

    pub fn render(&self, instructions: &mut Vec<EntityInstruction>, camera: &dyn Camera, client_tick: ClientTick) {
        if let Some(animation_data) = self.animation_data.as_ref() {
            animation_data.render(
                instructions,
                camera,
                true,
                self.entity_id,
                self.world_position,
                &self.animation_state,
                Direction::South,
                self.fade_state.calculate_alpha(client_tick),
                1.0,
            );
        }
    }

    pub fn get_entity_id(&self) -> EntityId {
        self.entity_id
    }

    pub fn get_tile_position(&self) -> TilePosition {
        self.tile_position
    }

    pub fn set_animation_data(&mut self, animation_data: Arc<AnimationData>) {
        self.animation_data = Some(animation_data);
    }

    pub fn get_entity_part_files(&self, library: &Library) -> Vec<String> {
        let resource_name = library.get::<ItemResource>(ItemResourceKey {
            item_id: self.item_id,
            is_identified: self.is_identified,
        });

        vec![format!("{ITEM_SPRITE_PREFIX}{resource_name}")]
    }
}
