use std::collections::HashMap;
use std::sync::Arc;

use bytemuck::{cast_slice, Pod, Zeroable};
use cgmath::{Point3, Vector2};
use wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, CompareFunction, DepthStencilState, Device, FragmentState, MultisampleState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, PushConstantRange, RenderPass, RenderPipeline, RenderPipelineDescriptor, Sampler,
    SamplerBindingType, ShaderModule, ShaderModuleDescriptor, ShaderStages, TextureFormat, TextureSampleType, TextureViewDimension,
    VertexState,
};

use super::PointShadowSubRenderer;
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::*;

const SHADER: ShaderModuleDescriptor = include_wgsl!("entity.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Constants {
    world: [[f32; 4]; 4],
    light_position: [f32; 4],
    texture_position: [f32; 2],
    texture_size: [f32; 2],
    depth_offset: f32,
    curvature: f32,
    mirror: u32,
}

pub struct EntityRenderer {
    device: Arc<Device>,
    nearest_sampler: Sampler,
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
}

impl EntityRenderer {
    pub fn new(device: Arc<Device>, output_depth_format: TextureFormat) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let nearest_sampler = create_new_sampler(&device, "entity nearest", SamplerType::Nearest);
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("entity"),
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
        let pipeline = Self::create_pipeline(&device, &shader_module, &bind_group_layout, output_depth_format);

        Self {
            device,
            nearest_sampler,
            bind_group_layout,
            pipeline,
        }
    }

    fn create_pipeline(
        device: &Device,
        shader_module: &ShaderModule,
        bind_group_layout: &BindGroupLayout,
        output_depth_format: TextureFormat,
    ) -> RenderPipeline {
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("entity"),
            bind_group_layouts: &[bind_group_layout, CubeFaceBuffer::bind_group_layout(device)],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        let mut constants = HashMap::new();
        constants.insert("near_plane".to_string(), NEAR_PLANE as f64);

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("entity"),
            layout: Some(&layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions {
                    constants: &constants,
                    ..Default::default()
                },
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: shader_module,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions {
                    constants: &constants,
                    ..Default::default()
                },
                targets: &[],
            }),
            multiview: None,
            primitive: PrimitiveState::default(),
            multisample: MultisampleState::default(),
            depth_stencil: Some(DepthStencilState {
                format: output_depth_format,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            cache: None,
        })
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn bind_pipeline(&self, render_target: &<PointShadowRenderer as Renderer>::Target, render_pass: &mut RenderPass) {
        let extent = render_target.texture.get_extent();

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_viewport(0.0, 0.0, extent.width as f32, extent.height as f32, 0.0, 1.0);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("entity renderer"))]
    pub fn render(
        &self,
        render_target: &mut <PointShadowRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        light_position: Point3<f32>,
        texture: &Texture,
        position: Point3<f32>,
        origin: Point3<f32>,
        scale: Vector2<f32>,
        cell_count: Vector2<usize>,
        cell_position: Vector2<usize>,
        mirror: bool,
    ) {
        if render_target.bind_sub_renderer(PointShadowSubRenderer::Entity) {
            self.bind_pipeline(render_target, render_pass);
        }

        let texture_extent = texture.get_extent();
        let size = Vector2::new(
            texture_extent.width as f32 * scale.x / 10.0,
            texture_extent.height as f32 * scale.y / 10.0,
        );

        let world_matrix = camera.billboard_matrix(position, origin, size);
        let texture_size = Vector2::new(1.0 / cell_count.x as f32, 1.0 / cell_count.y as f32);
        let texture_position = Vector2::new(texture_size.x * cell_position.x as f32, texture_size.y * cell_position.y as f32);
        let (depth_offset, curvature) = camera.calculate_depth_offset_and_curvature(&world_matrix, scale.x, scale.y);

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("entity"),
            layout: &self.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(texture.get_texture_view()),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&self.nearest_sampler),
                },
            ],
        });

        let push_constants = Constants {
            world: world_matrix.into(),
            light_position: light_position.to_homogeneous().into(),
            texture_position: texture_position.into(),
            texture_size: texture_size.into(),
            depth_offset,
            curvature,
            mirror: mirror as u32,
        };

        render_pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, cast_slice(&[push_constants]));
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.set_bind_group(1, render_target.face_bind_group(), &[]);
        render_pass.draw(0..6, 0..1);
    }
}
