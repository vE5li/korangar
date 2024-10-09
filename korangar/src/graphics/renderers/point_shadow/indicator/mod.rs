use std::sync::Arc;

use bytemuck::{cast_slice, Pod, Zeroable};
use cgmath::Point3;
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

const SHADER: ShaderModuleDescriptor = include_wgsl!("indicator.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Constants {
    light_position: [f32; 4],
    upper_left: [f32; 4],
    upper_right: [f32; 4],
    lower_left: [f32; 4],
    lower_right: [f32; 4],
}

pub struct IndicatorRenderer {
    device: Arc<Device>,
    nearest_sampler: Sampler,
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
}

impl IndicatorRenderer {
    pub fn new(device: Arc<Device>, output_depth_format: TextureFormat) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let nearest_sampler = create_new_sampler(&device, "indicator nearest", SamplerType::Nearest);
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("indicator"),
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
            label: Some("indicator"),
            bind_group_layouts: &[bind_group_layout, CubeFaceBuffer::bind_group_layout(device)],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("indicator"),
            layout: Some(&layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
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
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render ground indicator"))]
    pub fn render_ground_indicator(
        &self,
        render_target: &mut <PointShadowRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        _camera: &dyn Camera,
        light_position: Point3<f32>,
        texture: &Texture,
        upper_left: Point3<f32>,
        upper_right: Point3<f32>,
        lower_left: Point3<f32>,
        lower_right: Point3<f32>,
    ) {
        if render_target.bind_sub_renderer(PointShadowSubRenderer::Indicator) {
            self.bind_pipeline(render_pass);
        }

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("indicator"),
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
            light_position: light_position.to_homogeneous().into(),
            upper_left: upper_left.to_homogeneous().into(),
            upper_right: upper_right.to_homogeneous().into(),
            lower_left: lower_left.to_homogeneous().into(),
            lower_right: lower_right.to_homogeneous().into(),
        };

        render_pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, cast_slice(&[push_constants]));
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.set_bind_group(1, render_target.face_bind_group(), &[]);
        render_pass.draw(0..6, 0..1);
    }
}
