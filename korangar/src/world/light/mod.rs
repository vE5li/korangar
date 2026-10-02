use std::sync::Arc;

use cgmath::{Matrix4, MetricSpace, Point3, SquareMatrix, Vector3};
use korangar_collision::Sphere;
use ragnarok_formats::map::LightSource;
use ragnarok_packets::ClientTick;

#[cfg(feature = "debug")]
use crate::graphics::RenderOptions;
use crate::graphics::{
    Buffer, EntityInstruction, ModelInstruction, ModelVertex, PointLightInstruction, PointLightWithShadowInstruction, ScreenSize,
    TextureSet,
};
#[cfg(feature = "debug")]
use crate::renderer::MarkerRenderer;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;
use crate::world::{Entity, GroundItem, Map, ObjectKey, PointShadowCamera, PointShadowEntityCamera, ResourceSetBuffer};
use crate::{Camera, Color, NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS};

pub trait LightSourceExt {
    fn offset(&mut self, offset: Vector3<f32>);

    #[cfg(feature = "debug")]
    fn render_marker(&self, renderer: &mut impl MarkerRenderer, camera: &dyn Camera, marker_identifier: MarkerIdentifier, hovered: bool);
}

impl LightSourceExt for LightSource {
    fn offset(&mut self, offset: Vector3<f32>) {
        self.position += offset;
    }

    #[cfg(feature = "debug")]
    fn render_marker(&self, renderer: &mut impl MarkerRenderer, camera: &dyn Camera, marker_identifier: MarkerIdentifier, hovered: bool) {
        renderer.render_marker(camera, marker_identifier, self.position, hovered);
    }
}

/// Id of a point light. This should be unique and is used in the heuristic for
/// which light casts a shadow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointLightId(u32);

impl PointLightId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

#[derive(Clone, Debug)]
pub struct PointLight {
    pub id: PointLightId,
    pub position: Point3<f32>,
    pub color: Color,
    pub range: f32,
    pub final_range: f32,
}

impl PointLight {
    pub fn render(&self, instructions: &mut Vec<PointLightInstruction>) {
        instructions.push(PointLightInstruction {
            position: self.position,
            color: self.color,
            range: self.range,
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_with_shadows(
        &self,
        instructions: &mut Vec<PointLightWithShadowInstruction>,
        view_projection_matrices: [Matrix4<f32>; 6],
        view_matrices: [Matrix4<f32>; 6],
        model_texture_set: Arc<TextureSet>,
        model_vertex_buffer: Arc<Buffer<ModelVertex>>,
        model_index_buffer: Arc<Buffer<u32>>,
        entity_offset: [usize; 6],
        entity_count: [usize; 6],
        model_offset: [usize; 6],
        model_count: [usize; 6],
    ) {
        instructions.push(PointLightWithShadowInstruction {
            view_projection_matrices,
            view_matrices,
            position: self.position,
            color: self.color,
            range: self.range,
            model_texture_set,
            model_vertex_buffer,
            model_index_buffer,
            entity_offset,
            entity_count,
            model_offset,
            model_count,
        });
    }
}

/// Height above the entity position that is used as the center of an entity
/// when orienting it towards a point light and for culling.
const ENTITY_SHADOW_CENTER_HEIGHT: f32 = 7.0;
/// Conservative radius of the bounding sphere of an entity used for culling.
const ENTITY_SHADOW_RADIUS: f32 = 15.0;

pub struct PointShadowEntities<'a> {
    pub entities: &'a [Entity],
    pub dead_entities: &'a [Entity],
    pub ground_items: &'a [GroundItem],
    pub client_tick: ClientTick,
}

pub struct PointLightManager {
    point_lights: Vec<PointLight>,
    /// Field used for optimizing the memory allocation.
    have_shadows_this_frame: Vec<usize>,
    /// Field used for optimizing the memory allocation.
    have_no_shadows_this_frame: Vec<usize>,
    /// Field used for optimizing the memory allocation.
    had_shadows_last_frame: Vec<PointLightId>,
    /// Field used for optimizing the memory allocation.
    scored_point_lights: Vec<(usize, PointLightId, usize)>,
}

impl PointLightManager {
    pub fn new() -> Self {
        Self {
            point_lights: Vec::new(),
            have_shadows_this_frame: Vec::new(),
            have_no_shadows_this_frame: Vec::new(),
            had_shadows_last_frame: Vec::new(),
            scored_point_lights: Vec::new(),
        }
    }

    pub fn prepare(&mut self) {
        self.point_lights.clear();
    }

    pub fn register(&mut self, id: PointLightId, position: Point3<f32>, color: Color, range: f32) {
        self.point_lights.push(PointLight {
            id,
            position,
            color,
            range,
            final_range: range,
        });
    }

    pub fn register_fading(&mut self, id: PointLightId, position: Point3<f32>, color: Color, range: f32, final_range: f32) {
        self.point_lights.push(PointLight {
            id,
            position,
            color,
            range,
            final_range,
        });
    }

    pub fn clear(&mut self) {
        self.point_lights.clear();
        self.had_shadows_last_frame.clear();
    }

    fn score_point_light(&self, point_light: &PointLight) -> usize {
        const POINTS_FOR_CONSISTENCY: usize = 20;

        let size = point_light.final_range.sqrt();
        let intensity = [point_light.color.red + point_light.color.green + point_light.color.blue]
            .into_iter()
            .sum::<f32>();

        let constistency_bonus = match self.had_shadows_last_frame.contains(&point_light.id) {
            true => POINTS_FOR_CONSISTENCY,
            false => 0,
        };

        (size * intensity) as usize + constistency_bonus
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn create_point_light_set(&mut self, shadow_map_count: usize) -> PointLightSet<'_> {
        for (index, point_light) in self.point_lights.iter().enumerate() {
            self.scored_point_lights
                .push((index, point_light.id, self.score_point_light(point_light)))
        }

        self.have_shadows_this_frame.clear();
        self.have_no_shadows_this_frame.clear();
        self.had_shadows_last_frame.clear();

        self.scored_point_lights.sort_by_key(|right| std::cmp::Reverse(right.2));
        let mut scored_iterator = self.scored_point_lights.drain(..);

        for _ in 0..shadow_map_count {
            if let Some((index, id, _)) = scored_iterator.next() {
                self.have_shadows_this_frame.push(index);
                self.had_shadows_last_frame.push(id);
            }
        }

        for (index, ..) in scored_iterator {
            self.have_no_shadows_this_frame.push(index);
        }

        PointLightSet {
            point_lights: &self.point_lights,
            point_lights_with_shadows: &self.have_shadows_this_frame,
            point_lights_without_shadows: &self.have_no_shadows_this_frame,
        }
    }
}

/// A set of point lights with and without shadows.
pub struct PointLightSet<'a> {
    point_lights: &'a [PointLight],
    point_lights_with_shadows: &'a [usize],
    point_lights_without_shadows: &'a [usize],
}

impl PointLightSet<'_> {
    pub fn with_shadow_iterator(&self) -> impl Iterator<Item = &PointLight> {
        self.point_lights_with_shadows.iter().map(|index| &self.point_lights[*index])
    }

    pub fn without_shadow_iterator(&self) -> impl Iterator<Item = &PointLight> {
        self.point_lights_without_shadows.iter().map(|index| &self.point_lights[*index])
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_point_lights(&self, instructions: &mut Vec<PointLightInstruction>) {
        self.without_shadow_iterator()
            .for_each(|point_light| point_light.render(instructions));
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    #[allow(clippy::too_many_arguments)]
    pub fn render_point_lights_with_shadows(
        &self,
        map: &Map,
        point_shadow_camera: &mut PointShadowCamera,
        point_shadow_object_set_buffer: &mut ResourceSetBuffer<ObjectKey>,
        point_shadow_model_instructions: &mut Vec<ModelInstruction>,
        point_shadow_entity_instructions: &mut Vec<EntityInstruction>,
        point_light_with_shadow_instructions: &mut Vec<PointLightWithShadowInstruction>,
        point_shadow_entities: &PointShadowEntities,
        animation_timer_ms: f32,
        #[cfg(feature = "debug")] render_options: &RenderOptions,
    ) {
        for point_light in self.with_shadow_iterator() {
            point_shadow_camera.set_camera_position(point_light.position);

            let mut view_projection_matrices = [Matrix4::identity(); NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS];
            let mut view_matrices = [Matrix4::identity(); NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS];

            let mut entity_offsets = [0; NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS];
            let mut entity_counts = [0; NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS];
            let mut model_offsets = [0; NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS];
            let mut model_counts = [0; NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS];

            let object_set = map.cull_objects_in_sphere(
                Sphere::new(point_light.position, point_light.range),
                point_shadow_object_set_buffer,
                #[cfg(feature = "debug")]
                render_options.frustum_culling,
            );

            // TODO: Create an entity set, similar to the object set for better performance.
            for face_index in 0..6 {
                point_shadow_camera.change_direction(face_index);
                point_shadow_camera.generate_view_projection(ScreenSize::default());

                view_projection_matrices[face_index as usize] = point_shadow_camera.view_projection_matrix();
                (view_matrices[face_index as usize], _) = point_shadow_camera.view_projection_matrices();

                let model_offset = point_shadow_model_instructions.len();

                map.render_objects(
                    point_shadow_model_instructions,
                    &object_set,
                    animation_timer_ms,
                    point_shadow_camera,
                );

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.show_map))]
                map.render_ground(point_shadow_model_instructions);

                model_offsets[face_index as usize] = model_offset;
                model_counts[face_index as usize] = point_shadow_model_instructions.len() - model_offset;

                let entity_offset = point_shadow_entity_instructions.len();

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.show_ground_items))]
                Self::render_ground_item_shadows(
                    point_shadow_entity_instructions,
                    point_shadow_camera,
                    point_light,
                    point_shadow_entities.ground_items,
                    point_shadow_entities.client_tick,
                );

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.show_entities))]
                Self::render_entity_shadows(
                    point_shadow_entity_instructions,
                    point_shadow_camera,
                    point_light,
                    point_shadow_entities.entities,
                    point_shadow_entities.client_tick,
                );

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.show_entities))]
                Self::render_entity_shadows(
                    point_shadow_entity_instructions,
                    point_shadow_camera,
                    point_light,
                    point_shadow_entities.dead_entities,
                    point_shadow_entities.client_tick,
                );

                entity_offsets[face_index as usize] = entity_offset;
                entity_counts[face_index as usize] = point_shadow_entity_instructions.len() - entity_offset;
            }

            point_light.render_with_shadows(
                point_light_with_shadow_instructions,
                view_projection_matrices,
                view_matrices,
                map.get_texture_set().clone(),
                map.get_model_vertex_buffer().clone(),
                map.get_model_index_buffer().clone(),
                entity_offsets,
                entity_counts,
                model_offsets,
                model_counts,
            );
        }
    }

    /// Returns the center that an entity at the given position should be
    /// oriented towards the light with, if it can cast a shadow into the
    /// current face of the point shadow camera.
    fn entity_shadow_center(
        point_shadow_camera: &PointShadowCamera,
        point_light: &PointLight,
        position: Point3<f32>,
        scale: f32,
    ) -> Option<Point3<f32>> {
        let center = position + Vector3::unit_y() * (ENTITY_SHADOW_CENTER_HEIGHT * scale);
        let radius = ENTITY_SHADOW_RADIUS * scale;

        let in_range = point_light.position.distance(center) <= point_light.range + radius;
        let in_face = point_shadow_camera.sphere_intersects_face(center, radius);

        (in_range && in_face).then_some(center)
    }

    fn render_entity_shadows(
        instructions: &mut Vec<EntityInstruction>,
        point_shadow_camera: &PointShadowCamera,
        point_light: &PointLight,
        entities: &[Entity],
        client_tick: ClientTick,
    ) {
        for entity in entities {
            let (position, scale) = (entity.get_position(), entity.get_scale());

            if let Some(center) = Self::entity_shadow_center(point_shadow_camera, point_light, position, scale) {
                let camera = PointShadowEntityCamera::new(point_shadow_camera, center);
                entity.render(instructions, &camera, false, client_tick);
            }
        }
    }

    fn render_ground_item_shadows(
        instructions: &mut Vec<EntityInstruction>,
        point_shadow_camera: &PointShadowCamera,
        point_light: &PointLight,
        ground_items: &[GroundItem],
        client_tick: ClientTick,
    ) {
        for ground_item in ground_items {
            if let Some(center) = Self::entity_shadow_center(point_shadow_camera, point_light, ground_item.world_position, 1.0) {
                let camera = PointShadowEntityCamera::new(point_shadow_camera, center);
                ground_item.render(instructions, &camera, client_tick);
            }
        }
    }
}
