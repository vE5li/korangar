use cgmath::{Matrix4, Point3, SquareMatrix, Vector2, Vector3, Zero};
use korangar_util::collision::Sphere;
use ragnarok_formats::map::LightSource;
use ragnarok_packets::ClientTick;

#[cfg(feature = "debug")]
use crate::graphics::RenderSettings;
use crate::graphics::{ModelInstruction, PointLightInstruction, PointShadowCamera, PointShadowCasterInstruction};
use crate::interface::layout::{ScreenPosition, ScreenSize};
#[cfg(feature = "debug")]
use crate::renderer::MarkerRenderer;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;
use crate::world::{Map, ObjectKey, ResourceSetBuffer};
use crate::{Camera, Color, NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS};

/// Calculates the extent of a point light based on its range attribute.
pub fn point_light_extent(color: Color, range: f32) -> f32 {
    // The threshold of brightness at which we deem the light invisible.
    //
    // The current value will make objects in point lights cast shadows slightly
    // later than they become illuminated, but the dead-zone is very small so the
    // transition should never be noticeable in a real scenario.
    const VISIBILITY_THRESHOLD: f32 = 0.01;

    // If the color channel is less intense it will fade out more quickly, so we can
    // scale down the range to the strongest color channel.
    let color_intensity = [color.red, color.green, color.blue].into_iter().reduce(f32::max).unwrap();
    let adjusted_range = range * color_intensity;

    10.0 * (adjusted_range / VISIBILITY_THRESHOLD).ln()
}

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

/// ID of a point light. This should be unique and is used in the heuristic for
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
    pub fn render(&self, instructions: &mut Vec<PointLightInstruction>, camera: &dyn Camera) {
        let (screen_position, screen_size) = self.calculate_screen_position_and_size(camera);
        instructions.push(PointLightInstruction {
            position: self.position,
            color: self.color,
            screen_position,
            screen_size,
            range: self.range,
        });
    }

    pub fn render_with_shadows(
        &self,
        instructions: &mut Vec<PointShadowCasterInstruction>,
        camera: &dyn Camera,
        view_projection_matrices: [Matrix4<f32>; 6],
        entity_offset: [usize; 6],
        entity_count: [usize; 6],
        model_offset: [usize; 6],
        mode_count: [usize; 6],
    ) {
        let (screen_position, screen_size) = self.calculate_screen_position_and_size(camera);
        instructions.push(PointShadowCasterInstruction {
            view_projection_matrices,
            position: self.position,
            screen_position,
            screen_size,
            color: self.color,
            range: self.range,
            entity_offset,
            entity_count,
            model_offset,
            mode_count,
        });
    }

    fn calculate_screen_position_and_size(&self, camera: &dyn Camera) -> (ScreenPosition, ScreenSize) {
        let extent = point_light_extent(self.color, self.range);

        let corner_offset = (extent.powf(2.0) * 2.0).sqrt();
        let (mut top_left_position, mut bottom_right_position) = camera.billboard_coordinates(self.position, corner_offset);

        // A negative w means that the point light is behind the camera, but we still
        // need to render the quad since the point light has an effect on the
        // scene in front of the camera.
        top_left_position.w = top_left_position.w.abs();
        bottom_right_position.w = bottom_right_position.w.abs();

        camera.screen_position_size(top_left_position, bottom_right_position)
    }
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

        let constistency_bonus = self
            .had_shadows_last_frame
            .iter()
            .any(|id| *id == point_light.id)
            .then_some(POINTS_FOR_CONSISTENCY)
            .unwrap_or_default();

        (size * intensity) as usize + constistency_bonus
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn create_point_light_set(&mut self, shadow_map_count: usize) -> PointLightSet {
        for (index, point_light) in self.point_lights.iter().enumerate() {
            self.scored_point_lights
                .push((index, point_light.id, self.score_point_light(point_light)))
        }

        self.have_shadows_this_frame.clear();
        self.have_no_shadows_this_frame.clear();
        self.had_shadows_last_frame.clear();

        self.scored_point_lights.sort_by(|left, right| right.2.cmp(&left.2));
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
    pub fn render_point_lights(&self, instructions: &mut Vec<PointLightInstruction>, camera: &dyn Camera) {
        self.without_shadow_iterator()
            .for_each(|point_light| point_light.render(instructions, camera));
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_point_lights_with_shadows(
        &self,
        map: &Map,
        current_camera: &dyn Camera,
        point_shadow_camera: &mut PointShadowCamera,
        point_shadow_object_set_buffer: &mut ResourceSetBuffer<ObjectKey>,
        point_shadow_model_instructions: &mut Vec<ModelInstruction>,
        point_light_with_shadow_instructions: &mut Vec<PointShadowCasterInstruction>,
        client_tick: ClientTick,
        #[cfg(feature = "debug")] render_settings: &RenderSettings,
    ) {
        for point_light in self.with_shadow_iterator() {
            point_shadow_camera.set_camera_position(point_light.position);

            let mut view_projection_matrices = [Matrix4::identity(); NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS];
            let entity_offsets = [0; NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS];
            let entity_counts = [0; NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS];
            let mut model_offsets = [0; NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS];
            let mut mode_counts = [0; NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS];

            let extent = point_light_extent(point_light.color, point_light.range);
            let object_set = map.cull_objects_in_sphere(
                Sphere::new(point_light.position, extent),
                point_shadow_object_set_buffer,
                #[cfg(feature = "debug")]
                render_settings.frustum_culling,
            );

            // TODO: Create an entity set, similar to the object set for better performance.
            for face_index in 0..6 {
                point_shadow_camera.change_direction(face_index);
                point_shadow_camera.generate_view_projection(Vector2::zero());

                let (view_matrix, projection_matrix) = point_shadow_camera.view_projection_matrices();
                view_projection_matrices[face_index as usize] = projection_matrix * view_matrix;

                let model_offset = point_shadow_model_instructions.len();

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_objects))]
                map.render_objects(point_shadow_model_instructions, &object_set, client_tick);

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_map))]
                map.render_ground(point_shadow_model_instructions);

                model_offsets[face_index as usize] = model_offset;
                mode_counts[face_index as usize] = point_shadow_model_instructions.len() - model_offset;
            }

            point_light.render_with_shadows(
                point_light_with_shadow_instructions,
                current_camera,
                view_projection_matrices,
                entity_offsets,
                entity_counts,
                model_offsets,
                mode_counts,
            );
        }
    }
}
