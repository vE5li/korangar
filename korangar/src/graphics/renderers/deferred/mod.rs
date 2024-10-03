mod ambient;
#[cfg(feature = "debug")]
mod r#box;
#[cfg(feature = "debug")]
mod buffer;
#[cfg(feature = "debug")]
mod circle;
mod directional;
mod effect;
mod entity;
mod geometry;
mod indicator;
mod overlay;
mod point;
mod point_shadow;
mod rectangle;
mod sprite;
mod water;
mod water_light;

use std::sync::Arc;

#[cfg(feature = "debug")]
use cgmath::SquareMatrix;
use cgmath::{Matrix4, Point3, Vector2, Vector3};
#[cfg(feature = "debug")]
use circle::CircleRenderer;
use korangar_interface::application::FontSizeTrait;
#[cfg(feature = "debug")]
use korangar_util::collision::AABB;
#[cfg(feature = "debug")]
use ragnarok_formats::transform::Transform;
use ragnarok_packets::EntityId;
use renderers::texture::CubeTexture;
use wgpu::{Device, Queue, RenderPass, TextureFormat};

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
use self::point_shadow::PointLightWithShadowsRenderer;
use self::rectangle::RectangleRenderer;
use self::sprite::SpriteRenderer;
use self::water::WaterRenderer;
use self::water_light::WaterLightRenderer;
use crate::graphics::{
    Buffer, EntityRenderer as EntityRendererTrait, GeometryRenderer as GeometryRendererTrait, IndicatorRenderer as IndicatorRendererTrait,
    SpriteRenderer as SpriteRendererTrait, *,
};
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::loaders::{FontSize, TextureLoader};
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

#[derive(PartialEq, Eq)]
pub enum DeferredSubRenderer {
    Geometry,
    Entity,
    Water,
    AmbientLight,
    DirectionalLight,
    PointLight,
    PointLightWithShadows,
    WaterLight,
    Indicator,
    #[cfg(feature = "debug")]
    BoundingBox,
    #[cfg(feature = "debug")]
    Circle,
    #[cfg(feature = "debug")]
    Buffers,
    Overlay,
    Rectangle,
    Sprite,
    Effect,
}

pub struct DeferredRenderer {
    device: Arc<Device>,
    geometry_renderer: GeometryRenderer,
    entity_renderer: EntityRenderer,
    water_renderer: WaterRenderer,
    indicator_renderer: IndicatorRenderer,
    ambient_light_renderer: AmbientLightRenderer,
    directional_light_renderer: DirectionalLightRenderer,
    point_light_renderer: PointLightRenderer,
    point_light_with_shadows_renderer: PointLightWithShadowsRenderer,
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
    circle_renderer: CircleRenderer,
    #[cfg(feature = "debug")]
    tile_textures: TextureGroup,
    font_map: Arc<Texture>,
    walk_indicator: Arc<Texture>,
    surface_format: TextureFormat,
    dimensions: [u32; 2],
}

impl DeferredRenderer {
    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        texture_loader: &mut TextureLoader,
        surface_format: TextureFormat,
        dimensions: [u32; 2],
    ) -> Self {
        let output_diffuse_format = <Self as Renderer>::Target::output_diffuse_format();
        let output_normal_format = <Self as Renderer>::Target::output_normal_format();
        let output_water_format = <Self as Renderer>::Target::output_water_format();
        let output_depth_format = <Self as Renderer>::Target::output_depth_format();

        let geometry_renderer = GeometryRenderer::new(
            device.clone(),
            queue.clone(),
            output_diffuse_format,
            output_normal_format,
            output_water_format,
            output_depth_format,
        );
        let entity_renderer = EntityRenderer::new(
            device.clone(),
            queue.clone(),
            output_diffuse_format,
            output_normal_format,
            output_water_format,
            output_depth_format,
        );
        let water_renderer = WaterRenderer::new(
            device.clone(),
            queue.clone(),
            output_diffuse_format,
            output_normal_format,
            output_water_format,
            output_depth_format,
        );
        let indicator_renderer = IndicatorRenderer::new(
            device.clone(),
            queue.clone(),
            output_diffuse_format,
            output_normal_format,
            output_water_format,
            output_depth_format,
        );

        let ambient_light_renderer = AmbientLightRenderer::new(device.clone(), surface_format);
        let directional_light_renderer = DirectionalLightRenderer::new(device.clone(), queue.clone(), surface_format);
        let point_light_renderer = PointLightRenderer::new(device.clone(), queue.clone(), surface_format);
        let point_light_with_shadows_renderer = PointLightWithShadowsRenderer::new(device.clone(), queue.clone(), surface_format);
        let water_light_renderer = WaterLightRenderer::new(device.clone(), surface_format);
        let overlay_renderer = OverlayRenderer::new(device.clone(), surface_format);
        let rectangle_renderer = RectangleRenderer::new(device.clone(), surface_format);
        let sprite_renderer = SpriteRenderer::new(
            device.clone(),
            surface_format,
            #[cfg(feature = "debug")]
            texture_loader,
        );
        let effect_renderer = EffectRenderer::new(device.clone(), surface_format);
        #[cfg(feature = "debug")]
        let buffer_renderer = BufferRenderer::new(device.clone(), surface_format);
        #[cfg(feature = "debug")]
        let box_renderer = BoxRenderer::new(device.clone(), queue.clone(), surface_format);
        #[cfg(feature = "debug")]
        let circle_renderer = CircleRenderer::new(device.clone(), surface_format);

        let font_map = texture_loader.get("font.png").unwrap();
        let walk_indicator = texture_loader.get("grid.tga").unwrap();

        #[cfg(feature = "debug")]
        let tile_textures: Vec<Arc<Texture>> = vec![
            texture_loader.get("0.png").unwrap(),
            texture_loader.get("1.png").unwrap(),
            texture_loader.get("2.png").unwrap(),
            texture_loader.get("3.png").unwrap(),
            texture_loader.get("4.png").unwrap(),
            texture_loader.get("5.png").unwrap(),
            texture_loader.get("6.png").unwrap(),
        ];

        #[cfg(feature = "debug")]
        let tile_textures = TextureGroup::new(&device, "tile textures", tile_textures);

        Self {
            device,
            geometry_renderer,
            entity_renderer,
            water_renderer,
            indicator_renderer,
            ambient_light_renderer,
            directional_light_renderer,
            point_light_renderer,
            point_light_with_shadows_renderer,
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
            circle_renderer,
            #[cfg(feature = "debug")]
            tile_textures,
            font_map,
            walk_indicator,
            surface_format,
            dimensions,
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("reconfigure deferred pipeline"))]
    pub fn reconfigure_pipeline(&mut self, surface_format: TextureFormat, dimensions: [u32; 2], #[cfg(feature = "debug")] wireframe: bool) {
        self.geometry_renderer.recreate_pipeline(
            #[cfg(feature = "debug")]
            wireframe,
        );

        if self.surface_format != surface_format {
            self.surface_format = surface_format;

            self.ambient_light_renderer.recreate_pipeline(surface_format);
            self.directional_light_renderer.recreate_pipeline(surface_format);
            self.point_light_renderer.recreate_pipeline(surface_format);
            self.point_light_with_shadows_renderer.recreate_pipeline(surface_format);
            self.water_light_renderer.recreate_pipeline(surface_format);
            self.overlay_renderer.recreate_pipeline(surface_format);
            self.rectangle_renderer.recreate_pipeline(surface_format);
            self.sprite_renderer.recreate_pipeline(surface_format);
            self.effect_renderer.recreate_pipeline(surface_format);
            #[cfg(feature = "debug")]
            self.buffer_renderer.recreate_pipeline(surface_format);
            #[cfg(feature = "debug")]
            self.box_renderer.recreate_pipeline(surface_format);
        }

        self.dimensions = dimensions;
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("create deferred render target"))]
    pub fn create_render_target(&self) -> <Self as Renderer>::Target {
        <Self as Renderer>::Target::new(&self.device, self.dimensions)
    }

    pub fn render_water(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        vertex_buffer: &Buffer<WaterVertex>,
        day_timer: f32,
    ) {
        self.water_renderer
            .render(render_target, render_pass, camera, vertex_buffer, day_timer);
    }

    pub fn ambient_light(&self, render_target: &mut <Self as Renderer>::Target, render_pass: &mut RenderPass, color: Color) {
        self.ambient_light_renderer.render(render_target, render_pass, color);
    }

    pub fn directional_light(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        shadow_map: &Texture,
        light_matrix: Matrix4<f32>,
        direction: Vector3<f32>,
        color: Color,
        intensity: f32,
    ) {
        self.directional_light_renderer.render(
            render_target,
            render_pass,
            camera,
            shadow_map,
            light_matrix,
            direction,
            color,
            intensity,
        );
    }

    pub fn point_light(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        position: Point3<f32>,
        color: Color,
        range: f32,
    ) {
        self.point_light_renderer
            .render(render_target, render_pass, camera, position, color, range);
    }

    pub fn point_light_with_shadows(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        shadow_map: &CubeTexture,
        position: Point3<f32>,
        color: Color,
        range: f32,
    ) {
        self.point_light_with_shadows_renderer
            .render(render_target, render_pass, camera, shadow_map, position, color, range);
    }

    pub fn water_light(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        water_level: f32,
    ) {
        self.water_light_renderer.render(render_target, render_pass, camera, water_level);
    }

    pub fn overlay_interface(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        interface_texture: &Texture,
    ) {
        self.overlay_renderer.render(render_target, render_pass, interface_texture);
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
        render_pass: &mut RenderPass,
        position: ScreenPosition,
        size: ScreenSize,
        color: Color,
    ) {
        let window_size = self.get_window_size();

        self.rectangle_renderer
            .render(render_target, render_pass, window_size, position, size, color);
    }

    pub fn render_bar(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
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

        self.render_rectangle(render_target, render_pass, position - bar_offset, bar_size, color);
    }

    pub fn render_text(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        text: &str,
        mut position: ScreenPosition,
        color: Color,
        font_size: FontSize,
    ) {
        let window_size = self.get_window_size();

        for character in text.as_bytes() {
            let index = (*character as usize).saturating_sub(31);
            self.sprite_renderer.render_indexed(
                render_target,
                render_pass,
                &self.font_map,
                window_size,
                position,
                ScreenSize::uniform(font_size.get_value()),
                color,
                10,
                index,
                true,
            );
            position.left += font_size.get_value() / 2.0;
        }
    }

    pub fn render_damage_text(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
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
                render_pass,
                &self.font_map,
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
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        position: Point3<f32>,
        texture: &Texture,
        corner_screen_positions: [Vector2<f32>; 4],
        texture_coordinates: [Vector2<f32>; 4],
        offset: Vector2<f32>,
        angle: f32,
        color: Color,
    ) {
        let window_size = self.get_window_size();

        self.effect_renderer.render(
            render_target,
            render_pass,
            camera,
            position,
            texture,
            window_size,
            corner_screen_positions,
            texture_coordinates,
            offset,
            angle,
            color,
        );
    }

    #[cfg(feature = "debug")]
    pub fn render_overlay_tiles(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        vertex_buffer: &Buffer<ModelVertex>,
    ) {
        // FIX: This is broken on account of the TileTypes not storing their original
        // index. Should choose an index based on flags instead.
        self.render_geometry(
            render_target,
            render_pass,
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
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        transform: &Transform,
        bounding_box: &AABB,
        color: Color,
    ) {
        self.box_renderer
            .render(render_target, render_pass, camera, transform, bounding_box, color);
    }

    #[cfg(feature = "debug")]
    pub fn render_circle(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        position: Point3<f32>,
        color: Color,
        range: f32,
    ) {
        self.circle_renderer
            .render(render_target, render_pass, camera, position, color, range);
    }

    #[cfg(feature = "debug")]
    pub fn overlay_buffers(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        picker_texture: &Texture,
        shadow_map: &Texture,
        font_atlas: &Texture,
        point_shadow: &CubeTexture,
        render_settings: &RenderSettings,
    ) {
        self.buffer_renderer.render(
            render_target,
            render_pass,
            picker_texture,
            shadow_map,
            font_atlas,
            point_shadow,
            render_settings,
        );
    }
}

impl Renderer for DeferredRenderer {
    type Target = DeferredRenderTarget;
}

impl GeometryRendererTrait for DeferredRenderer {
    fn render_geometry(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        vertex_buffer: &Buffer<ModelVertex>,
        textures: &TextureGroup,
        world_matrix: Matrix4<f32>,
        time: f32,
    ) where
        Self: Renderer,
    {
        self.geometry_renderer
            .render(render_target, render_pass, camera, vertex_buffer, textures, world_matrix, time);
    }
}

impl EntityRendererTrait for DeferredRenderer {
    fn render_entity(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        texture: &Texture,
        position: Point3<f32>,
        origin: Point3<f32>,
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
            render_pass,
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
        render_pass: &mut RenderPass,
        texture: &Texture,
        position: ScreenPosition,
        size: ScreenSize,
        _screen_clip: ScreenClip,
        color: Color,
        smooth: bool,
    ) where
        Self: Renderer,
    {
        let window_size = self.get_window_size();

        self.sprite_renderer.render_indexed(
            render_target,
            render_pass,
            texture,
            window_size,
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
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        position: Point3<f32>,
        hovered: bool,
    ) where
        Self: Renderer,
    {
        let (top_left_position, bottom_right_position) = camera.billboard_coordinates(position, MarkerIdentifier::SIZE);

        if top_left_position.w >= 0.1 && bottom_right_position.w >= 0.1 {
            let (screen_position, screen_size) = camera.screen_position_size(top_left_position, bottom_right_position);

            self.sprite_renderer.render_marker(
                render_target,
                render_pass,
                marker_identifier,
                screen_position,
                screen_size,
                hovered,
            );
        }
    }
}

impl IndicatorRendererTrait for DeferredRenderer {
    fn render_walk_indicator(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        color: Color,
        upper_left: Point3<f32>,
        upper_right: Point3<f32>,
        lower_left: Point3<f32>,
        lower_right: Point3<f32>,
    ) where
        Self: Renderer,
    {
        self.indicator_renderer.render_ground_indicator(
            render_target,
            render_pass,
            camera,
            &self.walk_indicator,
            color,
            upper_left,
            upper_right,
            lower_left,
            lower_right,
        );
    }
}
