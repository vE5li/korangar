use std::sync::Arc;

use bytemuck::{cast_slice, Pod, Zeroable};
use cgmath::Point3;
use wgpu::{
    include_wgsl, ColorTargetState, ColorWrites, Device, FragmentState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PushConstantRange, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModule, ShaderModuleDescriptor, ShaderStages,
    TextureFormat, VertexState,
};

use super::{Camera, Color, DeferredRenderer, DeferredSubRenderer, Renderer, ALPHA_BLEND};

const SHADER: ShaderModuleDescriptor = include_wgsl!("circle.wgsl");

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Constants {
    position: [f32; 4],
    color: [f32; 4],
    screen_position: [f32; 2],
    screen_size: [f32; 2],
    size: f32,
}

pub struct CircleRenderer {
    device: Arc<Device>,
    shader_module: ShaderModule,
    pipeline: RenderPipeline,
}

impl CircleRenderer {
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
            label: Some("circle"),
            bind_group_layouts: &[],
            push_constant_ranges: &[PushConstantRange {
                stages: ShaderStages::VERTEX_FRAGMENT,
                range: 0..size_of::<Constants>() as _,
            }],
        });

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("circle"),
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

    #[cfg_attr(feature = "debug", korangar_debug::profile("render circle"))]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        position: Point3<f32>,
        color: Color,
        size: f32,
    ) {
        if render_target.bound_sub_renderer(DeferredSubRenderer::Circle) {
            self.bind_pipeline(render_pass);
        }

        let corner_offset = (size.powf(2.0) * 2.0).sqrt();
        let (top_left_position, bottom_right_position) = camera.billboard_coordinates(position, corner_offset);

        if top_left_position.w < 0.1 && bottom_right_position.w < 0.1 && camera.distance_to(position) > size {
            return;
        }

        let (screen_position, screen_size) = camera.screen_position_size(top_left_position, bottom_right_position);

        let push_constants = Constants {
            position: [position.x, position.y, position.z, 1.0],
            color: color.components_linear(),
            screen_position: screen_position.into(),
            screen_size: screen_size.into(),
            size,
        };

        render_pass.set_push_constants(ShaderStages::VERTEX_FRAGMENT, 0, cast_slice(&[push_constants]));
        render_pass.draw(0..6, 0..1);
    }
}
