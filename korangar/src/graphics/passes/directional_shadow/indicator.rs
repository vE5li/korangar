use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
    ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, Device, FragmentState, MultisampleState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor,
    ShaderStages, StencilState, TextureSampleType, TextureViewDimension, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, DirectionalShadowRenderPassContext, Drawer, RenderPassContext,
};
use crate::graphics::shader_compiler::ShaderCompiler;
use crate::graphics::{Capabilities, GlobalContext, IndicatorInstruction};

const DRAWER_NAME: &str = "directional shadow indicator";

pub(crate) struct DirectionalShadowIndicatorDrawer {
    bind_group: BindGroup,
    pipeline: RenderPipeline,
}

impl Drawer<{ BindGroupCount::Two }, { ColorAttachmentCount::One }, { DepthAttachmentCount::One }> for DirectionalShadowIndicatorDrawer {
    type Context = DirectionalShadowRenderPassContext;
    type DrawData<'data> = Option<&'data IndicatorInstruction>;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        shader_compiler: &ShaderCompiler,
        global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = shader_compiler.create_shader_module("directional_shadow", "indicator");

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

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some(DRAWER_NAME),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(global_context.walk_indicator_texture.get_texture_view()),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[
                Self::Context::bind_group_layout(device)[0],
                Self::Context::bind_group_layout(device)[1],
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
                    blend: None,
                    write_mask: ColorWrites::empty(),
                })],
            }),
            multiview: None,
            primitive: PrimitiveState::default(),
            multisample: MultisampleState::default(),
            depth_stencil: Some(DepthStencilState {
                format: render_pass_context.depth_attachment_output_format()[0],
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            cache: None,
        });

        Self { bind_group, pipeline }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        if draw_data.is_some() {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(2, &self.bind_group, &[]);
            pass.draw(0..6, 0..1);
        }
    }
}
