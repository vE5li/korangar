use wgpu::{
    include_wgsl, ColorTargetState, ColorWrites, CompareFunction, DepthBiasState, DepthStencilState, Device, FragmentState,
    MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor,
    ShaderModuleDescriptor, StencilState, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, GeometryRenderPassContext, RenderPassContext,
};
use crate::graphics::{Buffer, Capabilities, GlobalContext, WaterVertex};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/water.wgsl");
const DRAWER_NAME: &str = "geometry water";

pub(crate) struct GeometryWaterDrawer {
    pipeline: RenderPipeline,
}

impl Drawer<{ BindGroupCount::One }, { ColorAttachmentCount::Three }, { DepthAttachmentCount::One }> for GeometryWaterDrawer {
    type Context = GeometryRenderPassContext;
    type DrawData<'data> = &'data Buffer<WaterVertex>;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        _global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = device.create_shader_module(SHADER);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[Self::Context::bind_group_layout(device)[0]],
            push_constant_ranges: &[],
        });

        let color_attachment_formats = render_pass_context.color_attachment_formats();

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[WaterVertex::buffer_layout()],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[
                    Some(ColorTargetState {
                        format: color_attachment_formats[0],
                        blend: None,
                        write_mask: ColorWrites::default(),
                    }),
                    Some(ColorTargetState {
                        format: color_attachment_formats[1],
                        blend: None,
                        write_mask: ColorWrites::default(),
                    }),
                    Some(ColorTargetState {
                        format: color_attachment_formats[2],
                        blend: None,
                        write_mask: ColorWrites::default(),
                    }),
                ],
            }),
            multiview: None,
            primitive: Default::default(),
            multisample: MultisampleState {
                count: 4,
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: render_pass_context.depth_attachment_output_format()[0],
                depth_write_enabled: false,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            cache: None,
        });

        Self { pipeline }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, draw_data.slice(..));
        pass.draw(0..draw_data.count(), 0..1);
    }
}
