use std::collections::HashMap;
use std::mem::swap;
use std::sync::Arc;

use bytemuck::{cast_slice, Pod, Zeroable};
use cgmath::{Matrix2, Point3, Rad, Vector2};
use wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, BlendComponent, BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrites, Device, FragmentState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PushConstantRange, RenderPass, RenderPipeline, RenderPipelineDescriptor, Sampler,
    SamplerBindingType, ShaderModule, ShaderModuleDescriptor, ShaderStages, TextureFormat, TextureSampleType, TextureViewDimension,
    VertexState,
};

use super::{DeferredRenderer, DeferredSubRenderer};
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::{Camera, Color, Renderer, Texture};
use crate::interface::layout::ScreenSize;

const SHADER: ShaderModuleDescriptor = include_wgsl!("effect.wgsl");
/// These are the TOP5 combinations we currently find in the korean client
/// files and will preload at start.
const PRELOAD_PIPELINES: &[(BlendFactor, BlendFactor)] = &[
    (BlendFactor::SrcAlpha, BlendFactor::DstAlpha),
    (BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha),
    (BlendFactor::One, BlendFactor::Zero),
    (BlendFactor::Zero, BlendFactor::OneMinusSrcAlpha),
    (BlendFactor::OneMinusSrcAlpha, BlendFactor::OneMinusSrcAlpha),
];

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Constants {
    top_left: [f32; 2],
    bottom_left: [f32; 2],
    top_right: [f32; 2],
    bottom_right: [f32; 2],
    texture_top_left: [f32; 2],
    texture_bottom_left: [f32; 2],
    texture_top_right: [f32; 2],
    texture_bottom_right: [f32; 2],
    // Needs to be stored in two arrays,
    // or else we get alignment problems.
    color0: [f32; 2],
    color1: [f32; 2],
}

pub struct EffectRenderer {
    device: Arc<Device>,
    shader_module: ShaderModule,
    linear_sampler: Sampler,
    bind_group_layout: BindGroupLayout,
    surface_texture_format: TextureFormat,
    pipelines: HashMap<(BlendFactor, BlendFactor), RenderPipeline>,
}

impl EffectRenderer {
    pub fn new(device: Arc<Device>, surface_texture_format: TextureFormat) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let linear_sampler = create_new_sampler(&device, "effect linear", SamplerType::Linear);

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("effect"),
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

        let mut pipelines = HashMap::with_capacity(PRELOAD_PIPELINES.len());
        for (source_blend_factor, destination_blend_factor) in PRELOAD_PIPELINES.iter().copied() {
            let pipeline = Self::create_pipeline(
                &device,
                &shader_module,
                &bind_group_layout,
                surface_texture_format,
                source_blend_factor,
                destination_blend_factor,
            );
            pipelines.insert((source_blend_factor, destination_blend_factor), pipeline);
        }

        Self {
            device,
            shader_module,
            linear_sampler,
            bind_group_layout,
            surface_texture_format,
            pipelines,
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn recreate_pipeline(&mut self, surface_texture_format: TextureFormat) {
        self.surface_texture_format = surface_texture_format;

        let mut pipelines = HashMap::with_capacity(self.pipelines.len());
        swap(&mut self.pipelines, &mut pipelines);

        for ((source_blend_factor, destination_blend_factor), _) in pipelines.drain() {
            let pipeline = Self::create_pipeline(
                &self.device,
                &self.shader_module,
                &self.bind_group_layout,
                surface_texture_format,
                source_blend_factor,
                destination_blend_factor,
            );
            self.pipelines.insert((source_blend_factor, destination_blend_factor), pipeline);
        }
    }

    fn create_pipeline(
        device: &Device,
        shader_module: &ShaderModule,
        bind_group_layout: &BindGroupLayout,
        surface_texture_format: TextureFormat,
        source_blend_factor: BlendFactor,
        destination_blend_factor: BlendFactor,
    ) -> RenderPipeline {
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("effect"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("effect"),
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
                    format: surface_texture_format,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: source_blend_factor,
                            dst_factor: destination_blend_factor,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: source_blend_factor,
                            dst_factor: destination_blend_factor,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::default(),
                })],
            }),
            multiview: None,
            cache: None,
        })
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn bind_pipeline(&mut self, render_pass: &mut RenderPass, source_blend_factor: BlendFactor, destination_blend_factor: BlendFactor) {
        let pipeline = self
            .pipelines
            .entry((source_blend_factor, destination_blend_factor))
            .or_insert_with(|| {
                Self::create_pipeline(
                    &self.device,
                    &self.shader_module,
                    &self.bind_group_layout,
                    self.surface_texture_format,
                    source_blend_factor,
                    destination_blend_factor,
                )
            });
        render_pass.set_pipeline(pipeline);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render effect"))]
    pub fn render(
        &mut self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        position: Point3<f32>,
        texture: &Texture,
        window_size: ScreenSize,
        corner_screen_position: [Vector2<f32>; 4],
        texture_coordinates: [Vector2<f32>; 4],
        offset: Vector2<f32>,
        angle: Rad<f32>,
        color: Color,
        source_blend_factor: BlendFactor,
        destination_blend_factor: BlendFactor,
    ) {
        const EFFECT_ORIGIN: Vector2<f32> = Vector2::new(319.0, 291.0);

        if render_target.bound_sub_renderer(DeferredSubRenderer::Effect {
            source_blend_factor,
            destination_blend_factor,
        }) {
            self.bind_pipeline(render_pass, source_blend_factor, destination_blend_factor);
        }

        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let clip_space_position = projection_matrix * view_matrix * position.to_homogeneous();
        let screen_space_position = camera.clip_to_screen_space(clip_space_position);

        let half_screen = Vector2::new(window_size.width / 2.0, window_size.height / 2.0);
        let rotation_matrix = Matrix2::from_angle(angle);

        let corner_screen_position =
            corner_screen_position.map(|position| (rotation_matrix * position) + offset - EFFECT_ORIGIN - half_screen);

        let clip_space_positions = corner_screen_position.map(|position| {
            let normalized_screen_position = Vector2::new(
                (position.x / half_screen.x) * 0.5 + 0.5 + screen_space_position.x,
                (position.y / half_screen.y) * 0.5 + 0.5 + screen_space_position.y,
            );
            let clip_space_position = camera.screen_to_clip_space(normalized_screen_position);
            [clip_space_position.x, clip_space_position.y]
        });

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("effect"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.linear_sampler),
                },
            ],
        });

        let color = color.components_linear();

        let push_constants = Constants {
            top_left: clip_space_positions[0],
            bottom_left: clip_space_positions[2],
            top_right: clip_space_positions[1],
            bottom_right: clip_space_positions[3],
            texture_top_left: texture_coordinates[2].into(),
            texture_bottom_left: texture_coordinates[3].into(),
            texture_top_right: texture_coordinates[1].into(),
            texture_bottom_right: texture_coordinates[0].into(),
            color0: [color[0], color[1]],
            color1: [color[2], color[3]],
        };

        render_pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, cast_slice(&[push_constants]));
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
