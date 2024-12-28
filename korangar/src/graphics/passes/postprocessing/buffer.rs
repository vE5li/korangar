use wgpu::util::StagingBelt;
use wgpu::{
    include_wgsl, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BlendState, ColorTargetState, ColorWrites, CommandEncoder, Device, FragmentState, MultisampleState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor,
    ShaderModuleDescriptor, ShaderStages, TextureSampleType, TextureViewDimension, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, PostProcessingRenderPassContext, RenderPassContext,
};
use crate::graphics::settings::RenderSettings;
use crate::graphics::{Capabilities, GlobalContext, Prepare, RenderInstruction, Texture};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/buffer.wgsl");
const DRAWER_NAME: &str = "post processing buffer";

pub(crate) struct PostProcessingBufferDrawData<'a> {
    pub(crate) render_settings: &'a RenderSettings,
    pub(crate) debug_bind_group: &'a BindGroup,
}

pub(crate) struct PostProcessingBufferDrawer {
    bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
    pipeline: RenderPipeline,
}

impl Drawer<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }> for PostProcessingBufferDrawer {
    type Context = PostProcessingRenderPassContext;
    type DrawData<'data> = PostProcessingBufferDrawData<'data>;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = device.create_shader_module(SHADER);

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(DRAWER_NAME),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });

        let bind_group_layouts = Self::Context::bind_group_layout(device);

        let bind_group = Self::create_bind_group(device, &bind_group_layout, &global_context.solid_pixel_texture);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[
                bind_group_layouts[0],
                GlobalContext::debug_bind_group_layout(device),
                &bind_group_layout,
            ],
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
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::default(),
                })],
            }),
            primitive: PrimitiveState::default(),
            multisample: MultisampleState::default(),
            depth_stencil: None,
            multiview: None,
            cache: None,
        });

        Self {
            bind_group_layout,
            bind_group,
            pipeline,
        }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        if !draw_data.render_settings.show_buffers() {
            return;
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(1, draw_data.debug_bind_group, &[]);
        pass.set_bind_group(2, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

impl Prepare for PostProcessingBufferDrawer {
    fn prepare(&mut self, device: &Device, instructions: &RenderInstruction) {
        if let Some(font_map_texture) = instructions.font_map_texture {
            self.bind_group = Self::create_bind_group(device, &self.bind_group_layout, font_map_texture);
        }
    }

    fn upload(&mut self, _device: &Device, _staging_belt: &mut StagingBelt, _command_encoder: &mut CommandEncoder) {
        /* Nothing to do */
    }
}

impl PostProcessingBufferDrawer {
    fn create_bind_group(device: &Device, bind_group_layout: &BindGroupLayout, font_map_texture: &Texture) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(DRAWER_NAME),
            layout: bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(font_map_texture.get_texture_view()),
            }],
        })
    }
}
