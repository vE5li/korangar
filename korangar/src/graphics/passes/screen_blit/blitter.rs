use wgpu::{
    ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor, TextureSampleType,
    TextureViewDimension, VertexState, include_wgsl,
};

use crate::graphics::passes::screen_blit::ScreenBlitRenderPassContext;
use crate::graphics::passes::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer};
use crate::graphics::{AttachmentTexture, Capabilities, GlobalContext};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/blitter.wgsl");
const SHADER_SRGB: ShaderModuleDescriptor = include_wgsl!("shader/blitter_srgb.wgsl");
const DRAWER_NAME: &str = "screen blit blitter";

pub(crate) struct ScreenBlitBlitterDrawer {
    pipeline: RenderPipeline,
}

impl Drawer<{ BindGroupCount::None }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for ScreenBlitBlitterDrawer {
    type Context = ScreenBlitRenderPassContext;
    type DrawData<'data> = &'data AttachmentTexture;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        global_context: &GlobalContext,
        _render_pass_context: &Self::Context,
    ) -> Self {
        let surface_texture_format = global_context.surface_texture_format;

        let shader_module = match surface_texture_format.is_srgb() {
            true => device.create_shader_module(SHADER_SRGB),
            false => device.create_shader_module(SHADER),
        };

        let label = format!("{DRAWER_NAME} {surface_texture_format:?}");

        let texture_bind_group_layout = AttachmentTexture::bind_group_layout(
            device,
            TextureViewDimension::D2,
            TextureSampleType::Float { filterable: true },
            false,
        );

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(&label),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(&label),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: surface_texture_format,
                    blend: None,
                    write_mask: ColorWrites::default(),
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self { pipeline }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, draw_data.get_bind_group(), &[]);
        pass.draw(0..3, 0..1);
    }
}
