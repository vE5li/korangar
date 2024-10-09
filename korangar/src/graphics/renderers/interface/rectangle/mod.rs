use std::sync::Arc;

use bytemuck::{cast_slice, Pod, Zeroable};
use wgpu::{
    include_wgsl, ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PushConstantRange, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModule,
    ShaderModuleDescriptor, ShaderStages, TextureFormat, VertexState,
};

use super::{InterfaceRenderer, InterfaceSubRenderer};
use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};
use crate::{Color, Renderer, INTERFACE_ATTACHMENT_BLEND};

const SHADER: ShaderModuleDescriptor = include_wgsl!("rectangle.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Constants {
    screen_position: [f32; 2],
    screen_size: [f32; 2],
    screen_clip: [f32; 4],
    corner_radius: [f32; 4],
    color: [f32; 4],
    aspect_ratio: f32,
}

pub struct RectangleRenderer {
    pipeline: RenderPipeline,
}

impl RectangleRenderer {
    pub fn new(device: Arc<Device>, output_texture_format: TextureFormat) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let pipeline = Self::create_pipeline(&device, &shader_module, output_texture_format);

        Self { pipeline }
    }

    fn create_pipeline(device: &Device, shader_module: &ShaderModule, output_texture_format: TextureFormat) -> RenderPipeline {
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
            multisample: MultisampleState {
                count: 4,
                ..Default::default()
            },
            fragment: Some(FragmentState {
                module: shader_module,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: output_texture_format,
                    blend: Some(INTERFACE_ATTACHMENT_BLEND),
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
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        window_size: ScreenSize,
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        screen_clip: ScreenClip,
        corner_radius: CornerRadius,
        color: Color,
    ) {
        if render_target.bound_sub_renderer(InterfaceSubRenderer::Rectangle) {
            self.bind_pipeline(render_pass);
        }

        let half_screen = window_size / 2.0;
        let screen_position = screen_position / half_screen;
        let screen_size = screen_size / half_screen;

        let pixel_size = 1.0 / window_size.height;
        let corner_radius = corner_radius * pixel_size;

        let push_constants = Constants {
            screen_position: screen_position.into(),
            screen_size: screen_size.into(),
            screen_clip: screen_clip.into(),
            corner_radius: corner_radius.into(),
            color: color.components_linear(),
            aspect_ratio: window_size.height / window_size.width,
        };

        render_pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, cast_slice(&[push_constants]));
        render_pass.draw(0..6, 0..1);
    }
}
