use wgpu::{
    ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor, ShaderModuleDescriptor, TextureSampleType,
    TextureViewDimension, VertexState, include_wgsl,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, PostProcessingRenderPassContext, RenderPassContext,
};
use crate::graphics::shader_compiler::ShaderCompiler;
use crate::graphics::{AttachmentTexture, Capabilities, GlobalContext};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/fxaa.wgsl");
const DRAWER_NAME: &str = "post processing fxaa";

pub(crate) struct PostProcessingFxaaDrawer {
    pipeline: RenderPipeline,
}

impl Drawer<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for PostProcessingFxaaDrawer {
    type Context = PostProcessingRenderPassContext;
    type DrawData<'data> = &'data AttachmentTexture;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        _shader_compiler: &ShaderCompiler,
        _global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = device.create_shader_module(SHADER);

        let pass_bind_group_layouts = Self::Context::bind_group_layout(device);

        let input_texture_bind_group_layout = AttachmentTexture::bind_group_layout(
            device,
            TextureViewDimension::D2,
            TextureSampleType::Float { filterable: true },
            false,
        );

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[pass_bind_group_layouts[0], &input_texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
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
                    format: render_pass_context.color_attachment_formats()[0],
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
        pass.set_bind_group(1, draw_data.get_bind_group(), &[]);
        pass.draw(0..3, 0..1);
    }
}
