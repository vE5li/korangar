mod ambient;
#[cfg(feature = "debug")]
mod r#box;
#[cfg(feature = "debug")]
mod buffer;
mod directional;
mod effect;
mod entity;
mod geometry;
mod indicator;
mod overlay;
mod point;
mod rectangle;
mod sprite;
mod water;
mod water_light;

use std::sync::Arc;

#[cfg(feature = "debug")]
use cgmath::SquareMatrix;
use cgmath::{Matrix4, Vector2, Vector3};
use procedural::profile;
use vulkano::device::{DeviceOwned, Queue};
use vulkano::format::Format;
use vulkano::image::Image;
use vulkano::ordered_passes_renderpass;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{RenderPass, Subpass};

use self::ambient::AmbientLightRenderer;
#[cfg(feature = "debug")]
use self::r#box::BoxRenderer;
#[cfg(feature = "debug")]
use self::buffer::BufferRenderer;
use self::directional::DirectionalLightRenderer;
use self::effect::EffectRenderer;
use self::entity::EntityRenderer;
use self::geometry::GeometryRenderer;
use self::indicator::IndicatorRenderer;
use self::overlay::OverlayRenderer;
use self::point::PointLightRenderer;
use self::rectangle::RectangleRenderer;
use self::sprite::SpriteRenderer;
use self::water::WaterRenderer;
use self::water_light::WaterLightRenderer;
use super::SubpassAttachments;
use crate::graphics::{
    EntityRenderer as EntityRendererTrait, GeometryRenderer as GeometryRendererTrait, IndicatorRenderer as IndicatorRendererTrait,
    SpriteRenderer as SpriteRendererTrait, *,
};
use crate::interface::{ScreenClip, ScreenPosition, ScreenSize};
use crate::loaders::{GameFileLoader, TextureLoader};
use crate::network::EntityId;
#[cfg(feature = "debug")]
use crate::world::{BoundingBox, MarkerIdentifier};

#[derive(PartialEq, Eq)]
pub enum DeferredSubrenderer {
    Geometry,
    Entity,
    Water,
    AmbientLight,
    DirectionalLight,
    PointLight,
    WaterLight,
    Indicator,
    #[cfg(feature = "debug")]
    BoundingBox,
    #[cfg(feature = "debug")]
    Buffers,
    Overlay,
    Rectangle,
    Sprite,
    Effect,
}

pub struct DeferredRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    geometry_renderer: GeometryRenderer,
    entity_renderer: EntityRenderer,
    water_renderer: WaterRenderer,
    indicator_renderer: IndicatorRenderer,
    ambient_light_renderer: AmbientLightRenderer,
    directional_light_renderer: DirectionalLightRenderer,
    point_light_renderer: PointLightRenderer,
    water_light_renderer: WaterLightRenderer,
    overlay_renderer: OverlayRenderer,
    rectangle_renderer: RectangleRenderer,
    sprite_renderer: SpriteRenderer,
    effect_renderer: EffectRenderer,
    #[cfg(feature = "debug")]
    buffer_renderer: BufferRenderer,
    #[cfg(feature = "debug")]
    box_renderer: BoxRenderer,
    #[cfg(feature = "debug")]
    tile_textures: [Arc<ImageView>; 7],
    font_map: Arc<ImageView>,
    walk_indicator: Arc<ImageView>,
    dimensions: [u32; 2],
}

unsafe impl Send for DeferredRenderer {}
unsafe impl Sync for DeferredRenderer {}

impl DeferredRenderer {
    const fn deferred_subpass() -> SubpassAttachments {
        SubpassAttachments { color: 3, depth: 1 }
    }

    const fn lighting_subpass() -> SubpassAttachments {
        SubpassAttachments { color: 1, depth: 0 }
    }

    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        buffer_allocator: &mut BufferAllocator,
        game_file_loader: &mut GameFileLoader,
        texture_loader: &mut TextureLoader,
        queue: Arc<Queue>,
        swapchain_format: Format,
        viewport: Viewport,
        dimensions: [u32; 2],
    ) -> Self {
        let device = memory_allocator.device().clone();
        let render_pass = ordered_passes_renderpass!(device,
            attachments: {
                output: {
                    format: swapchain_format,
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
                diffuse: {
                    format: Format::R32G32B32A32_SFLOAT,
                    samples: 4,
                    load_op: Clear,
                    store_op: Store,
                },
                normal: {
                    format: Format::R16G16B16A16_SFLOAT,
                    samples: 4,
                    load_op: Clear,
                    store_op: Store,
                },
                water: {
                    format: Format::R8G8B8A8_UNORM,
                    samples: 4,
                    load_op: Clear,
                    store_op: Store,
                },
                depth: {
                    format: Format::D32_SFLOAT,
                    samples: 4,
                    load_op: Clear,
                    store_op: Store,
                }
            },
            passes: [
                {
                    color: [diffuse, normal, water],
                    depth_stencil: {depth},
                    input: []
                },
                {
                    color: [output],
                    depth_stencil: {},
                    input: [diffuse, normal, water, depth]
                }
            ]
        )
        .unwrap();

        let geometry_subpass = Subpass::from(render_pass.clone(), 0).unwrap();
        let lighting_subpass = Subpass::from(render_pass.clone(), 1).unwrap();

        let geometry_renderer = GeometryRenderer::new(memory_allocator.clone(), geometry_subpass.clone(), viewport.clone());
        let entity_renderer = EntityRenderer::new(memory_allocator.clone(), geometry_subpass.clone(), viewport.clone());
        let water_renderer = WaterRenderer::new(memory_allocator.clone(), geometry_subpass.clone(), viewport.clone());
        let indicator_renderer = IndicatorRenderer::new(memory_allocator.clone(), geometry_subpass, viewport.clone());
        let ambient_light_renderer = AmbientLightRenderer::new(memory_allocator.clone(), lighting_subpass.clone(), viewport.clone());
        let directional_light_renderer =
            DirectionalLightRenderer::new(memory_allocator.clone(), lighting_subpass.clone(), viewport.clone());
        let point_light_renderer = PointLightRenderer::new(memory_allocator.clone(), lighting_subpass.clone(), viewport.clone());
        let water_light_renderer = WaterLightRenderer::new(memory_allocator.clone(), lighting_subpass.clone(), viewport.clone());
        let overlay_renderer = OverlayRenderer::new(memory_allocator.clone(), lighting_subpass.clone(), viewport.clone());
        let rectangle_renderer = RectangleRenderer::new(memory_allocator.clone(), lighting_subpass.clone(), viewport.clone());
        let sprite_renderer = SpriteRenderer::new(
            memory_allocator.clone(),
            lighting_subpass.clone(),
            viewport.clone(),
            #[cfg(feature = "debug")]
            game_file_loader,
            #[cfg(feature = "debug")]
            texture_loader,
        );
        let effect_renderer = EffectRenderer::new(memory_allocator.clone(), lighting_subpass.clone(), viewport.clone());
        #[cfg(feature = "debug")]
        let buffer_renderer = BufferRenderer::new(memory_allocator.clone(), lighting_subpass.clone(), viewport.clone());
        #[cfg(feature = "debug")]
        let box_renderer = BoxRenderer::new(memory_allocator.clone(), buffer_allocator, lighting_subpass, viewport);

        let font_map = texture_loader.get("font.png", game_file_loader).unwrap();
        let walk_indicator = texture_loader.get("grid.tga", game_file_loader).unwrap();

        #[cfg(feature = "debug")]
        let tile_textures = [
            texture_loader.get("0.png", game_file_loader).unwrap(),
            texture_loader.get("1.png", game_file_loader).unwrap(),
            texture_loader.get("2.png", game_file_loader).unwrap(),
            texture_loader.get("3.png", game_file_loader).unwrap(),
            texture_loader.get("4.png", game_file_loader).unwrap(),
            texture_loader.get("5.png", game_file_loader).unwrap(),
            texture_loader.get("6.png", game_file_loader).unwrap(),
        ];

        Self {
            memory_allocator,
            queue,
            render_pass,
            geometry_renderer,
            entity_renderer,
            water_renderer,
            indicator_renderer,
            ambient_light_renderer,
            directional_light_renderer,
            point_light_renderer,
            water_light_renderer,
            overlay_renderer,
            rectangle_renderer,
            sprite_renderer,
            effect_renderer,
            #[cfg(feature = "debug")]
            buffer_renderer,
            #[cfg(feature = "debug")]
            box_renderer,
            #[cfg(feature = "debug")]
            tile_textures,
            font_map,
            walk_indicator,
            dimensions,
        }
    }

    #[profile("re-create deferred pipeline")]
    pub fn recreate_pipeline(&mut self, viewport: Viewport, dimensions: [u32; 2], #[cfg(feature = "debug")] wireframe: bool) {
        let device = self.memory_allocator.device().clone();
        let geometry_subpass = Subpass::from(self.render_pass.clone(), 0).unwrap();
        let lighting_subpass = Subpass::from(self.render_pass.clone(), 1).unwrap();

        self.geometry_renderer.recreate_pipeline(
            device.clone(),
            geometry_subpass.clone(),
            viewport.clone(),
            #[cfg(feature = "debug")]
            wireframe,
        );
        self.entity_renderer
            .recreate_pipeline(device.clone(), geometry_subpass.clone(), viewport.clone());
        self.water_renderer
            .recreate_pipeline(device.clone(), geometry_subpass.clone(), viewport.clone());
        self.indicator_renderer
            .recreate_pipeline(device.clone(), geometry_subpass, viewport.clone());
        self.ambient_light_renderer
            .recreate_pipeline(device.clone(), lighting_subpass.clone(), viewport.clone());
        self.directional_light_renderer
            .recreate_pipeline(device.clone(), lighting_subpass.clone(), viewport.clone());
        self.point_light_renderer
            .recreate_pipeline(device.clone(), lighting_subpass.clone(), viewport.clone());
        self.water_light_renderer
            .recreate_pipeline(device.clone(), lighting_subpass.clone(), viewport.clone());
        self.overlay_renderer
            .recreate_pipeline(device.clone(), lighting_subpass.clone(), viewport.clone());
        self.rectangle_renderer
            .recreate_pipeline(device.clone(), lighting_subpass.clone(), viewport.clone());
        self.sprite_renderer
            .recreate_pipeline(device.clone(), lighting_subpass.clone(), viewport.clone());
        self.effect_renderer
            .recreate_pipeline(device.clone(), lighting_subpass.clone(), viewport.clone());
        #[cfg(feature = "debug")]
        self.buffer_renderer
            .recreate_pipeline(device.clone(), lighting_subpass.clone(), viewport.clone());
        #[cfg(feature = "debug")]
        self.box_renderer.recreate_pipeline(device, lighting_subpass, viewport);
        self.dimensions = dimensions;
    }

    #[profile("create deferred render target")]
    pub fn create_render_target(&self, swapchain_image: Arc<Image>) -> <Self as Renderer>::Target {
        <Self as Renderer>::Target::new(
            self.memory_allocator.clone(),
            self.queue.clone(),
            self.render_pass.clone(),
            swapchain_image,
            self.dimensions,
        )
    }

    pub fn render_water(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        vertex_buffer: Subbuffer<[WaterVertex]>,
        day_timer: f32,
    ) {
        self.water_renderer.render(render_target, camera, vertex_buffer, day_timer);
    }

    pub fn ambient_light(&self, render_target: &mut <Self as Renderer>::Target, color: Color) {
        self.ambient_light_renderer.render(render_target, color);
    }

    pub fn directional_light(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        light_image: Arc<ImageView>,
        light_matrix: Matrix4<f32>,
        direction: Vector3<f32>,
        color: Color,
        intensity: f32,
    ) {
        self.directional_light_renderer
            .render(render_target, camera, light_image, light_matrix, direction, color, intensity);
    }

    pub fn point_light(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        position: Vector3<f32>,
        color: Color,
        range: f32,
    ) {
        self.point_light_renderer.render(render_target, camera, position, color, range);
    }

    pub fn water_light(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, water_level: f32) {
        self.water_light_renderer.render(render_target, camera, water_level);
    }

    pub fn overlay_interface(&self, render_target: &mut <Self as Renderer>::Target, interface_image: Arc<ImageView>) {
        self.overlay_renderer.render(render_target, interface_image);
    }

    fn get_window_size(&self) -> ScreenSize {
        ScreenSize {
            width: self.dimensions[0] as f32,
            height: self.dimensions[1] as f32,
        }
    }

    pub fn render_rectangle(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        position: ScreenPosition,
        size: ScreenSize,
        color: Color,
    ) {
        self.rectangle_renderer
            .render(render_target, self.get_window_size(), position, size, color);
    }

    pub fn render_bar(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        position: ScreenPosition,
        size: ScreenSize,
        color: Color,
        maximum: f32,
        current: f32,
    ) {
        let bar_offset = ScreenSize::only_width(size.width / 2.0);
        let bar_size = ScreenSize {
            width: (size.width / maximum) * current,
            height: size.height,
        };

        self.render_rectangle(render_target, position - bar_offset, bar_size, color);
    }

    pub fn render_text(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        text: &str,
        mut position: ScreenPosition,
        color: Color,
        font_size: f32,
    ) {
        let window_size = self.get_window_size();

        for character in text.as_bytes() {
            let index = (*character as usize).saturating_sub(31);
            self.sprite_renderer.render_indexed(
                render_target,
                self.font_map.clone(),
                window_size,
                position,
                ScreenSize::uniform(font_size),
                color,
                10,
                index,
                true,
            );
            position.left += font_size / 2.0;
        }
    }

    pub fn render_damage_text(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        text: &str,
        mut position: ScreenPosition,
        color: Color,
        font_size: f32,
    ) {
        let window_size = self.get_window_size();

        for character in text.as_bytes() {
            let index = (*character as usize).saturating_sub(31);
            self.sprite_renderer.render_indexed(
                render_target,
                self.font_map.clone(),
                window_size,
                position,
                ScreenSize::uniform(font_size),
                color,
                10,
                index,
                true,
            );
            position.left += font_size / 2.0;
        }
    }

    pub fn render_effect(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        texture: Arc<ImageView>,
        screen_positions: [Vector2<f32>; 4],
        texture_coordinates: [Vector2<f32>; 4],
        screen_space_position: Vector2<f32>,
        offset: Vector2<f32>,
        angle: f32,
        color: Color,
    ) {
        let window_size = self.get_window_size();

        self.effect_renderer.render(
            render_target,
            texture,
            window_size,
            screen_positions,
            texture_coordinates,
            screen_space_position,
            offset,
            angle,
            color,
        );
    }

    #[cfg(feature = "debug")]
    pub fn render_overlay_tiles(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        vertex_buffer: Subbuffer<[ModelVertex]>,
    ) {
        // FIX: This is broken on account of the TileTypes not storing their original
        // index. Should choose an index based on flags instead.
        self.render_geometry(
            render_target,
            camera,
            vertex_buffer,
            &self.tile_textures,
            Matrix4::identity(),
            0.0,
        );
    }

    #[cfg(feature = "debug")]
    pub fn render_bounding_box(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        transform: &Transform,
        bounding_box: &BoundingBox,
        color: Color,
    ) {
        self.box_renderer.render(render_target, camera, transform, bounding_box, color);
    }

    #[cfg(feature = "debug")]
    pub fn overlay_buffers(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        picker_image: Arc<ImageView>,
        light_image: Arc<ImageView>,
        font_atlas: Arc<ImageView>,
        render_settings: &RenderSettings,
    ) {
        self.buffer_renderer
            .render(render_target, picker_image, light_image, font_atlas, render_settings);
    }
}

impl Renderer for DeferredRenderer {
    type Target = DeferredRenderTarget;
}

impl GeometryRendererTrait for DeferredRenderer {
    fn render_geometry(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        vertex_buffer: Subbuffer<[ModelVertex]>,
        textures: &[Arc<ImageView>],
        world_matrix: Matrix4<f32>,
        time: f32,
    ) where
        Self: Renderer,
    {
        self.geometry_renderer
            .render(render_target, camera, vertex_buffer, textures, world_matrix, time);
    }
}

impl EntityRendererTrait for DeferredRenderer {
    fn render_entity(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        texture: Arc<ImageView>,
        position: Vector3<f32>,
        origin: Vector3<f32>,
        scale: Vector2<f32>,
        cell_count: Vector2<usize>,
        cell_position: Vector2<usize>,
        mirror: bool,
        _entity_id: EntityId,
    ) where
        Self: Renderer,
    {
        self.entity_renderer.render(
            render_target,
            camera,
            texture,
            position,
            origin,
            scale,
            cell_count,
            cell_position,
            mirror,
        );
    }
}

impl SpriteRendererTrait for DeferredRenderer {
    fn render_sprite(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        texture: Arc<ImageView>,
        position: ScreenPosition,
        size: ScreenSize,
        _screen_clip: ScreenClip,
        color: Color,
        smooth: bool,
    ) where
        Self: Renderer,
    {
        self.sprite_renderer.render_indexed(
            render_target,
            texture,
            self.get_window_size(),
            position,
            size,
            color,
            1,
            0,
            smooth,
        );
    }
}

#[cfg(feature = "debug")]
impl MarkerRenderer for DeferredRenderer {
    fn render_marker(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        position: Vector3<f32>,
        hovered: bool,
    ) where
        Self: Renderer,
    {
        let (top_left_position, bottom_right_position) = camera.billboard_coordinates(position, MarkerIdentifier::SIZE);

        if top_left_position.w >= 0.1 && bottom_right_position.w >= 0.1 {
            let (screen_position, screen_size) = camera.screen_position_size(bottom_right_position, top_left_position); // WHY ARE THESE INVERTED ???

            self.sprite_renderer
                .render_marker(render_target, marker_identifier, screen_position, screen_size, hovered);
        }
    }
}

impl IndicatorRendererTrait for DeferredRenderer {
    fn render_walk_indicator(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        color: Color,
        upper_left: Vector3<f32>,
        upper_right: Vector3<f32>,
        lower_left: Vector3<f32>,
        lower_right: Vector3<f32>,
    ) where
        Self: Renderer,
    {
        self.indicator_renderer.render_ground_indicator(
            render_target,
            camera,
            self.walk_indicator.clone(),
            color,
            upper_left,
            upper_right,
            lower_left,
            lower_right,
        );
    }
}
