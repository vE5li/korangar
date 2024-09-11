use std::sync::Arc;

use bytemuck::{cast_slice, Pod, Zeroable};
use wgpu::{
    include_wgsl, ColorTargetState, ColorWrites, Device, FragmentState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PushConstantRange, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderStages,
    TextureFormat, VertexState,
};

use super::DeferredSubRenderer;
use crate::graphics::*;
use crate::interface::layout::{ScreenPosition, ScreenSize};

const SHADER: ShaderModuleDescriptor = include_wgsl!("rectangle.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Constants {
    color: [f32; 4],
    screen_position: [f32; 2],
    screen_size: [f32; 2],
}

pub struct RectangleRenderer {
    device: Arc<Device>,
    shader_module: ShaderModule,
    pipeline: RenderPipeline,
}

impl RectangleRenderer {
    pub fn new(device: Arc<Device>, surface_format: TextureFormat) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let pipeline = Self::create_pipeline(&device, &shader_module, surface_format);

        Self {
            device,
            shader_module,
            pipeline,
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn recreate_pipeline(&mut self, surface_format: TextureFormat) {
        self.pipeline = Self::create_pipeline(&self.device, &self.shader_module, surface_format);
    }

    fn create_pipeline(device: &Device, shader_module: &ShaderModule, surface_format: TextureFormat) -> RenderPipeline {
        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("rectangle"),
            bind_group_layouts: &[],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("rectangle"),
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

    #[cfg_attr(feature = "debug", korangar_debug::profile("render rectangle"))]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        window_size: ScreenSize,
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        color: Color,
    ) {
        if render_target.bound_sub_renderer(DeferredSubRenderer::Rectangle) {
            self.bind_pipeline(render_pass);
        }

        let screen_position = screen_position / window_size;
        let screen_size = screen_size / window_size;

        let push_constants = Constants {
            color: color.components_linear(),
            screen_position: screen_position.into(),
            screen_size: screen_size.into(),
        };

        render_pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, cast_slice(&[push_constants]));
        render_pass.draw(0..6, 0..1);
    }
}
