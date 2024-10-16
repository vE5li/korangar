use wgpu::{
    include_wgsl, BlendState, ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor,
    VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, RenderPassContext, ScreenRenderPassContext,
};
use crate::graphics::GlobalContext;

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/overlay.wgsl");
const DRAWER_NAME: &str = "screen overlay";

pub(crate) struct ScreenOverlayDrawer {
    pipeline: RenderPipeline,
}

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for ScreenOverlayDrawer {
    type Context = ScreenRenderPassContext;
    type DrawData<'data> = Option<()>;

    fn new(device: &Device, _queue: &Queue, _global_context: &GlobalContext, render_pass_context: &Self::Context) -> Self {
        let shader_module = device.create_shader_module(SHADER);

        let bind_group_layouts = Self::Context::bind_group_layout(device);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[bind_group_layouts[0], bind_group_layouts[1]],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: render_pass_context.color_attachment_formats()[0],
                    blend: Some(BlendState::ALPHA_BLENDING),
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

    fn draw(&mut self, pass: &mut RenderPass<'_>, _draw_data: Self::DrawData<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.draw(0..3, 0..1);
    }
}
