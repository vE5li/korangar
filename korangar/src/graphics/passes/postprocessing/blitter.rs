use std::collections::HashMap;

use wgpu::{
    include_wgsl, BindGroupLayout, BlendState, ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor,
    ShaderModule, ShaderModuleDescriptor, TextureSampleType, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, PostProcessingRenderPassContext, RenderPassContext,
};
use crate::graphics::{AttachmentTexture, Capabilities, GlobalContext, Msaa};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/blitter.wgsl");
const SHADER_MSAA: ShaderModuleDescriptor = include_wgsl!("shader/blitter_msaa.wgsl");
const DRAWER_NAME: &str = "post processing blitter";

pub(crate) struct PostProcessingBlitterDrawer {
    pipeline_x1: RenderPipeline,
    pipeline_x2: RenderPipeline,
    pipeline_x4: RenderPipeline,
    pipeline_x8: RenderPipeline,
    pipeline_x16: RenderPipeline,
}

impl Drawer<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for PostProcessingBlitterDrawer {
    type Context = PostProcessingRenderPassContext;
    type DrawData<'data> = &'data AttachmentTexture;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        _global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = device.create_shader_module(SHADER);
        let msaa_module = device.create_shader_module(SHADER_MSAA);

        let pass_bind_group_layouts = Self::Context::bind_group_layout(device);

        let pipeline_x1 = Self::create_pipeline(device, render_pass_context, &shader_module, &pass_bind_group_layouts, Msaa::Off);
        let pipeline_x2 = Self::create_pipeline(device, render_pass_context, &msaa_module, &pass_bind_group_layouts, Msaa::X2);
        let pipeline_x4 = Self::create_pipeline(device, render_pass_context, &msaa_module, &pass_bind_group_layouts, Msaa::X4);
        let pipeline_x8 = Self::create_pipeline(device, render_pass_context, &msaa_module, &pass_bind_group_layouts, Msaa::X8);
        let pipeline_x16 = Self::create_pipeline(device, render_pass_context, &msaa_module, &pass_bind_group_layouts, Msaa::X16);

        Self {
            pipeline_x1,
            pipeline_x2,
            pipeline_x4,
            pipeline_x8,
            pipeline_x16,
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        match draw_data.get_texture().sample_count() {
            1 => {
                pass.set_pipeline(&self.pipeline_x1);
            }
            2 => {
                pass.set_pipeline(&self.pipeline_x2);
            }
            4 => {
                pass.set_pipeline(&self.pipeline_x4);
            }
            8 => {
                pass.set_pipeline(&self.pipeline_x8);
            }
            16 => {
                pass.set_pipeline(&self.pipeline_x16);
            }
            sample_count => panic!("Unsupported sample count: {sample_count}"),
        }

        pass.set_bind_group(1, draw_data.get_bind_group(), &[]);
        pass.draw(0..3, 0..1);
    }
}

impl PostProcessingBlitterDrawer {
    fn create_pipeline(
        device: &Device,
        render_pass_context: &PostProcessingRenderPassContext,
        shader_module: &ShaderModule,
        pass_bind_group_layouts: &[&BindGroupLayout; 1],
        msaa: Msaa,
    ) -> RenderPipeline {
        let label = format!("{DRAWER_NAME} {msaa}");

        let texture_bind_group_layout = AttachmentTexture::bind_group_layout(
            device,
            TextureSampleType::Float {
                filterable: !msaa.multisampling_activated(),
            },
            msaa.multisampling_activated(),
        );

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(&label),
            bind_group_layouts: &[pass_bind_group_layouts[0], &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let mut constants = HashMap::new();
        constants.insert("SAMPLE_COUNT".to_string(), f64::from(msaa.sample_count()));

        device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(&label),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants: &constants,
                    ..Default::default()
                },
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants: &constants,
                    ..Default::default()
                },
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
        })
    }
}
