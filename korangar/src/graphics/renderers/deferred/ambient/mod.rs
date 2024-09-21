use std::sync::Arc;

use bytemuck::checked::cast_slice;
use bytemuck::{Pod, Zeroable};
use wgpu::{
    include_wgsl, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, ColorTargetState, ColorWrites, Device, FragmentState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PushConstantRange, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderStages,
    TextureFormat, TextureSampleType, TextureViewDimension, VertexState,
};

use super::{Color, DeferredRenderer, DeferredSubRenderer, Renderer, LIGHT_ATTACHMENT_BLEND};

const SHADER: ShaderModuleDescriptor = include_wgsl!("ambient.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Constants {
    color: [f32; 4],
}

pub struct AmbientLightRenderer {
    device: Arc<Device>,
    shader_module: ShaderModule,
    bind_group_layout: BindGroupLayout,
    pipeline: RenderPipeline,
}

impl AmbientLightRenderer {
    pub fn new(device: Arc<Device>, surface_format: TextureFormat) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("ambient light"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: true,
                },
                count: None,
            }],
        });

        let pipeline = Self::create_pipeline(&device, &shader_module, &bind_group_layout, surface_format);

        Self {
            device,
            shader_module,
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
            label: Some("ambient light"),
            bind_group_layouts: &[bind_group_layout],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::FRAGMENT,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("ambient light"),
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
                    blend: Some(LIGHT_ATTACHMENT_BLEND),
                    write_mask: ColorWrites::default(),
                })],
            }),
            multiview: None,
            cache: None,
        })
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn bind_pipeline(&self, render_target: &<DeferredRenderer as Renderer>::Target, render_pass: &mut RenderPass) {
        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("ambient light"),
            layout: &self.bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(render_target.diffuse_buffer.get_texture_view()),
            }],
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render ambient light"))]
    pub fn render(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, render_pass: &mut RenderPass, color: Color) {
        if render_target.bound_sub_renderer(DeferredSubRenderer::AmbientLight) {
            self.bind_pipeline(render_target, render_pass);
        }

        let push_constants = Constants {
            color: color.components_linear(),
        };

        render_pass.set_push_constants(ShaderStages::FRAGMENT, 0, cast_slice(&[push_constants]));
        render_pass.draw(0..3, 0..1);
    }
}
