use std::sync::Arc;

use bytemuck::{cast_slice, Pod, Zeroable};
use cgmath::Vector2;
use wgpu::{
    include_wgsl, ColorTargetState, ColorWrites, CompareFunction, DepthStencilState, Device, FragmentState, MultisampleState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PushConstantRange, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderStages, TextureFormat, VertexState,
};

use super::{PickerRenderer, PickerSubRenderer, PickerTarget, Renderer};
use crate::world::MarkerIdentifier;

const SHADER: ShaderModuleDescriptor = include_wgsl!("marker.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Constants {
    screen_position: [f32; 2],
    screen_size: [f32; 2],
    identifier: u32,
}

pub struct MarkerRenderer {
    pipeline: RenderPipeline,
}

impl MarkerRenderer {
    pub fn new(device: Arc<Device>, output_color_format: TextureFormat, output_depth_format: TextureFormat) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let pipeline = Self::create_pipeline(device, &shader_module, output_color_format, output_depth_format);

        Self { pipeline }
    }

    fn create_pipeline(
        device: Arc<Device>,
        shader_module: &ShaderModule,
        output_color_format: TextureFormat,
        output_depth_format: TextureFormat,
    ) -> RenderPipeline {
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("marker"),
            bind_group_layouts: &[],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("marker"),
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
                targets: &[Some(ColorTargetState {
                    format: output_color_format,
                    blend: None,
                    write_mask: ColorWrites::default(),
                })],
            }),
            multiview: None,
            primitive: PrimitiveState::default(),
            multisample: MultisampleState::default(),
            depth_stencil: Some(DepthStencilState {
                format: output_depth_format,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Greater,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            cache: None,
        })
    }

    #[korangar_debug::profile]
    fn bind_pipeline(&self, render_pass: &mut RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
    }

    #[korangar_debug::profile("render marker")]
    pub fn render(
        &self,
        render_target: &mut <PickerRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        screen_position: Vector2<f32>,
        screen_size: Vector2<f32>,
        marker_identifier: MarkerIdentifier,
    ) {
        if render_target.bound_sub_renderer(PickerSubRenderer::Marker) {
            self.bind_pipeline(render_pass);
        }

        let picker_target = PickerTarget::Marker(marker_identifier);

        let push_constants = Constants {
            screen_position: screen_position.into(),
            screen_size: screen_size.into(),
            identifier: picker_target.into(),
        };

        render_pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, cast_slice(&[push_constants]));
        render_pass.draw(0..6, 0..1);
    }
}
