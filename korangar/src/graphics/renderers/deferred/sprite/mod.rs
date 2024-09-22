use std::sync::Arc;

use bytemuck::{cast_slice, Pod, Zeroable};
use cgmath::Vector2;
use wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, ColorTargetState, ColorWrites, Device, FragmentState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PushConstantRange, RenderPass, RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType, ShaderModule,
    ShaderModuleDescriptor, ShaderStages, TextureFormat, TextureSampleType, TextureViewDimension, VertexState,
};

use super::{Color, DeferredRenderer, DeferredSubRenderer, Renderer, Texture, ALPHA_BLEND};
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::interface::layout::{ScreenPosition, ScreenSize};
#[cfg(feature = "debug")]
use crate::loaders::TextureLoader;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

const SHADER: ShaderModuleDescriptor = include_wgsl!("sprite.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Constants {
    color: [f32; 4],
    screen_position: [f32; 2],
    screen_size: [f32; 2],
    texture_position: [f32; 2],
    texture_size: [f32; 2],
}

pub struct SpriteRenderer {
    device: Arc<Device>,
    shader_module: ShaderModule,
    #[cfg(feature = "debug")]
    object_marker_texture: Arc<Texture>,
    #[cfg(feature = "debug")]
    light_source_marker_texture: Arc<Texture>,
    #[cfg(feature = "debug")]
    sound_source_marker_texture: Arc<Texture>,
    #[cfg(feature = "debug")]
    effect_source_marker_texture: Arc<Texture>,
    #[cfg(feature = "debug")]
    entity_marker_texture: Arc<Texture>,
    #[cfg(feature = "debug")]
    shadow_marker_texture: Arc<Texture>,
    nearest_sampler: Sampler,
    linear_sampler: Sampler,
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
}

impl SpriteRenderer {
    pub fn new(device: Arc<Device>, surface_format: TextureFormat, #[cfg(feature = "debug")] texture_loader: &mut TextureLoader) -> Self {
        let shader_module = device.create_shader_module(SHADER);

        #[cfg(feature = "debug")]
        let object_marker_texture = texture_loader.get("object.png").unwrap();
        #[cfg(feature = "debug")]
        let light_source_marker_texture = texture_loader.get("light.png").unwrap();
        #[cfg(feature = "debug")]
        let sound_source_marker_texture = texture_loader.get("sound.png").unwrap();
        #[cfg(feature = "debug")]
        let effect_source_marker_texture = texture_loader.get("effect.png").unwrap();
        #[cfg(feature = "debug")]
        let entity_marker_texture = texture_loader.get("entity.png").unwrap();
        #[cfg(feature = "debug")]
        let shadow_marker_texture = texture_loader.get("shadow.png").unwrap();

        let nearest_sampler = create_new_sampler(&device, "sprite nearest", SamplerType::Nearest);
        let linear_sampler = create_new_sampler(&device, "sprite linear", SamplerType::Linear);
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("directional light"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline = Self::create_pipeline(&device, &shader_module, &bind_group_layout, surface_format);

        Self {
            device,
            shader_module,
            #[cfg(feature = "debug")]
            object_marker_texture,
            #[cfg(feature = "debug")]
            light_source_marker_texture,
            #[cfg(feature = "debug")]
            sound_source_marker_texture,
            #[cfg(feature = "debug")]
            effect_source_marker_texture,
            #[cfg(feature = "debug")]
            entity_marker_texture,
            #[cfg(feature = "debug")]
            shadow_marker_texture,
            nearest_sampler,
            linear_sampler,
            bind_group_layout,
            pipeline,
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn recreate_pipeline(&mut self, surface_format: TextureFormat) {
        self.pipeline = Self::create_pipeline(&self.device, &self.shader_module, &self.bind_group_layout, surface_format);
    }

    fn create_pipeline(
        device: &Device,
        shader_module: &ShaderModule,
        bind_group_layout: &BindGroupLayout,
        surface_format: TextureFormat,
    ) -> RenderPipeline {
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("sprite"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("sprite"),
            layout: Some(&layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(FragmentState {
                module: shader_module,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(ALPHA_BLEND),
                    write_mask: ColorWrites::default(),
                })],
            }),
            multiview: None,
            cache: None,
        })
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn bind_pipeline(&self, render_pass: &mut RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
    }

    fn build(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        texture: &Texture,
        screen_position: Vector2<f32>,
        screen_size: Vector2<f32>,
        texture_position: Vector2<f32>,
        texture_size: Vector2<f32>,
        color: Color,
        smooth: bool,
    ) {
        if render_target.bound_sub_renderer(DeferredSubRenderer::Sprite) {
            self.bind_pipeline(render_pass);
        }

        let sampler = match smooth {
            true => &self.linear_sampler,
            false => &self.nearest_sampler,
        };

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("directional light"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
        });

        let push_constants = Constants {
            color: color.components_linear(),
            screen_position: screen_position.into(),
            screen_size: screen_size.into(),
            texture_position: texture_position.into(),
            texture_size: texture_size.into(),
        };

        render_pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, cast_slice(&[push_constants]));
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render sprite indexed"))]
    pub fn render_indexed(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        texture: &Texture,
        window_size: ScreenSize,
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        color: Color,
        column_count: usize,
        cell_index: usize,
        smooth: bool,
    ) {
        let screen_position = Vector2 {
            x: screen_position.left / window_size.width,
            y: screen_position.top / window_size.height,
        };
        let screen_size = Vector2 {
            x: screen_size.width / window_size.width,
            y: screen_size.height / window_size.height,
        };

        let unit = 1.0 / column_count as f32;
        let offset_x = unit * (cell_index % column_count) as f32;
        let offset_y = unit * (cell_index / column_count) as f32;

        self.build(
            render_target,
            render_pass,
            texture,
            screen_position,
            screen_size,
            Vector2::new(offset_x, offset_y),
            Vector2::new(unit, unit),
            color,
            smooth,
        );
    }

    #[cfg(feature = "debug")]
    #[korangar_debug::profile("render marker")]
    pub fn render_marker(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        marker_identifier: MarkerIdentifier,
        screen_position: Vector2<f32>,
        screen_size: Vector2<f32>,
        hovered: bool,
    ) {
        let (texture, color) = match marker_identifier {
            MarkerIdentifier::Object(..) if hovered => (&self.object_marker_texture, Color::rgb_u8(235, 180, 52)),
            MarkerIdentifier::Object(..) => (&self.object_marker_texture, Color::rgb_u8(235, 103, 52)),
            MarkerIdentifier::LightSource(..) if hovered => (&self.light_source_marker_texture, Color::rgb_u8(150, 52, 235)),
            MarkerIdentifier::LightSource(..) => (&self.light_source_marker_texture, Color::rgb_u8(52, 235, 217)),
            MarkerIdentifier::SoundSource(..) if hovered => (&self.sound_source_marker_texture, Color::rgb_u8(128, 52, 235)),
            MarkerIdentifier::SoundSource(..) => (&self.sound_source_marker_texture, Color::rgb_u8(235, 52, 140)),
            MarkerIdentifier::EffectSource(..) if hovered => (&self.effect_source_marker_texture, Color::rgb_u8(235, 52, 52)),
            MarkerIdentifier::EffectSource(..) => (&self.effect_source_marker_texture, Color::rgb_u8(52, 235, 156)),
            MarkerIdentifier::Particle(..) if hovered => return,
            MarkerIdentifier::Particle(..) => return,
            MarkerIdentifier::Entity(..) if hovered => (&self.entity_marker_texture, Color::rgb_u8(235, 92, 52)),
            MarkerIdentifier::Entity(..) => (&self.entity_marker_texture, Color::rgb_u8(189, 235, 52)),
            MarkerIdentifier::Shadow(..) if hovered => (&self.shadow_marker_texture, Color::rgb_u8(200, 200, 200)),
            MarkerIdentifier::Shadow(..) => (&self.shadow_marker_texture, Color::rgb_u8(170, 170, 170)),
        };

        self.build(
            render_target,
            render_pass,
            texture,
            screen_position,
            screen_size,
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 1.0),
            color,
            true,
        );
    }
}
