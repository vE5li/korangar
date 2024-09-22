use std::sync::Arc;

use bytemuck::{cast_slice, Pod, Zeroable};
use cgmath::{EuclideanSpace, Matrix4, Point3};
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, CompareFunction, DepthStencilState, Device, FragmentState, MultisampleState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, PushConstantRange, RenderPass, RenderPipeline, RenderPipelineDescriptor, SamplerBindingType,
    ShaderModule, ShaderModuleDescriptor, ShaderStages, TextureFormat, VertexState,
};

use crate::graphics::renderers::point_shadow::PointShadowSubRenderer;
use crate::graphics::renderers::sampler::{create_new_sampler, SamplerType};
use crate::graphics::*;

const SHADER: ShaderModuleDescriptor = include_wgsl!("geometry.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Constants {
    world: [[f32; 4]; 4],
    light_position: [f32; 4],
}

pub struct GeometryRenderer {
    bind_group: BindGroup,
    pipeline: RenderPipeline,
}

impl GeometryRenderer {
    pub fn new(device: Arc<Device>, output_depth_format: TextureFormat) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let nearest_sampler = create_new_sampler(&device, "geometry nearest", SamplerType::Nearest);
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("geometry"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("geometry"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Sampler(&nearest_sampler),
            }],
        });

        let pipeline = Self::create_pipeline(&device, &shader_module, &bind_group_layout, output_depth_format);

        Self { bind_group, pipeline }
    }

    fn create_pipeline(
        device: &Device,
        shader_module: &ShaderModule,
        bind_group_layout: &BindGroupLayout,
        output_depth_format: TextureFormat,
    ) -> RenderPipeline {
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("geometry"),
            bind_group_layouts: &[
                bind_group_layout,
                TextureGroup::bind_group_layout(device),
                CubeFaceBuffer::bind_group_layout(device),
            ],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("geometry"),
            layout: Some(&layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[ModelVertex::buffer_layout()],
            },
            fragment: Some(FragmentState {
                module: shader_module,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions::default(),
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
    fn bind_pipeline(&self, render_pass: &mut RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("geometry renderer"))]
    pub fn render(
        &self,
        render_target: &mut <PointShadowRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        _camera: &dyn Camera,
        light_position: Point3<f32>,
        vertex_buffer: &Buffer<ModelVertex>,
        textures: &TextureGroup,
        world_matrix: Matrix4<f32>,
        time: f32,
    ) {
        if render_target.bind_sub_renderer(PointShadowSubRenderer::Geometry) {
            self.bind_pipeline(render_pass);
        }

        let push_constants = Constants {
            world: world_matrix.into(),
            light_position: light_position.to_vec().extend(time).into(),
        };

        render_pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, cast_slice(&[push_constants]));
        render_pass.set_bind_group(1, &textures.bind_group(), &[]);
        render_pass.set_bind_group(2, render_target.face_bind_group(), &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..vertex_buffer.count(), 0..1);
    }
}
