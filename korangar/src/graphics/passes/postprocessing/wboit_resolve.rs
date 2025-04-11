use wgpu::{
    BlendComponent, BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor,
    ShaderModuleDescriptor, TextureSampleType, VertexState, include_wgsl,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, PostProcessingRenderPassContext, RenderPassContext,
};
use crate::graphics::{AttachmentTexture, Capabilities, GlobalContext};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/wboit_resolve.wgsl");
const SHADER_MSAA: ShaderModuleDescriptor = include_wgsl!("shader/wboit_resolve_msaa.wgsl");

const DRAWER_NAME: &str = "post processing wboit resolve";

pub(crate) struct PostProcessingWboitResolveDrawData<'a> {
    pub(crate) accumulation_texture: &'a AttachmentTexture,
    pub(crate) revealage_texture: &'a AttachmentTexture,
}

pub(crate) struct PostProcessingWboitResolveDrawer {
    pipeline: RenderPipeline,
}

impl Drawer<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for PostProcessingWboitResolveDrawer {
    type Context = PostProcessingRenderPassContext;
    type DrawData<'data> = PostProcessingWboitResolveDrawData<'data>;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let msaa_activated = global_context.msaa.multisampling_activated();

        let shader_module = if msaa_activated {
            device.create_shader_module(SHADER_MSAA)
        } else {
            device.create_shader_module(SHADER)
        };

        let color_texture_format = render_pass_context.color_attachment_formats()[0];

        let texture_bind_group_layout = AttachmentTexture::bind_group_layout(
            device,
            TextureSampleType::Float {
                filterable: !msaa_activated,
            },
            msaa_activated,
        );

        let pass_bind_group_layouts = Self::Context::bind_group_layout(device);
        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[pass_bind_group_layouts[0], &texture_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let constants = &[("MSAA_SAMPLE_COUNT", f64::from(global_context.msaa.sample_count()))];

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants,
                    ..Default::default()
                },
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants,
                    ..Default::default()
                },
                targets: &[Some(ColorTargetState {
                    format: color_texture_format,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::OneMinusSrcAlpha,
                            dst_factor: BlendFactor::SrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::OneMinusSrcAlpha,
                            dst_factor: BlendFactor::SrcAlpha,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
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
        pass.set_bind_group(1, draw_data.accumulation_texture.get_bind_group(), &[]);
        pass.set_bind_group(2, draw_data.revealage_texture.get_bind_group(), &[]);
        pass.draw(0..3, 0..1);
    }
}
