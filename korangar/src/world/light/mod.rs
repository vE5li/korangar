use std::sync::Arc;

use cgmath::{Point3, Vector3};
use ragnarok_formats::map::LightSource;
use wgpu::RenderPass;

#[cfg(feature = "debug")]
use crate::graphics::MarkerRenderer;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;
use crate::{Camera, Color, CubeTexture, DeferredRenderer, Renderer};

/// Calculates the extend of a point light based on its range attribute.
pub fn point_light_extent(color: Color, range: f32) -> f32 {
    // The threshold of brightness at which we deem the light invisible.
    //
    // The current value will make objects in point lights cast shadows slightly
    // later than they become illuminated, but the dead-zone is very small so the
    // transition should never be noticeable in a real scenario.
    const VISIBILITY_THRESHOLD: f32 = 0.01;

    // If the color channel is less intense it will fade out more quickly, so we can
    // scale down the range to the strongest color channel.
    let color_intesity = [color.red, color.green, color.blue].into_iter().reduce(f32::max).unwrap();
    let adjusted_range = range * color_intesity;

    10.0 * (adjusted_range / VISIBILITY_THRESHOLD).ln()
}

pub trait LightSourceExt {
    fn offset(&mut self, offset: Vector3<f32>);

    #[cfg(feature = "debug")]
    fn render_marker<T>(
        &self,
        render_target: &mut T::Target,
        render_pass: &mut RenderPass,
        renderer: &T,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        hovered: bool,
    ) where
        T: Renderer + MarkerRenderer;
}

impl LightSourceExt for LightSource {
    fn offset(&mut self, offset: Vector3<f32>) {
        self.position += offset;
    }

    #[cfg(feature = "debug")]
    fn render_marker<T>(
        &self,
        render_target: &mut T::Target,
        render_pass: &mut RenderPass,
        renderer: &T,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        hovered: bool,
    ) where
        T: Renderer + MarkerRenderer,
    {
        renderer.render_marker(render_target, render_pass, camera, marker_identifier, self.position, hovered);
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
    fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
    ) {
        renderer.point_light(render_target, render_pass, camera, self.position, self.color, self.range);
    }

    fn render_with_shadows(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
        shadow_map: &CubeTexture,
    ) {
        renderer.point_light_with_shadows(
            render_target,
            render_pass,
            camera,
            shadow_map,
            self.position,
            self.color,
            self.range,
        );
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
    pub fn render_point_lights(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
        shadow_maps: &[Arc<CubeTexture>],
    ) {
        self.with_shadow_iterator()
            .zip(shadow_maps.iter())
            .for_each(|(point_light, shadow_map)| {
                point_light.render_with_shadows(render_target, render_pass, renderer, camera, shadow_map)
            });

        self.without_shadow_iterator()
            .for_each(|point_light| point_light.render(render_target, render_pass, renderer, camera));
    }
}
