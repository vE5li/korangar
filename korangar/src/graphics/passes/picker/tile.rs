use wgpu::{
    ColorTargetState, ColorWrites, CompareFunction, DepthStencilState, Device, FragmentState, IndexFormat, MultisampleState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor,
    ShaderModuleDescriptor, VertexState, include_wgsl,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, PickerRenderPassContext, RenderPassContext,
};
use crate::graphics::picker_target::PickerValueType;
use crate::graphics::{Buffer, Capabilities, GlobalContext, TileVertex};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/tile.wgsl");
const DRAWER_NAME: &str = "picker tile";

pub(crate) struct PickerTileDrawData<'a> {
    pub(crate) vertex_buffer: &'a Buffer<TileVertex>,
    pub(crate) index_buffer: &'a Buffer<u32>,
}

pub(crate) struct PickerTileDrawer {
    pipeline: RenderPipeline,
}

impl Drawer<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::One }> for PickerTileDrawer {
    type Context = PickerRenderPassContext;
    type DrawData<'draw> = PickerTileDrawData<'draw>;

    fn new(
        _capabilities: &Capabilities,
        device: &Device,
        _queue: &Queue,
        _global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self {
        let shader_module = device.create_shader_module(SHADER);

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[Self::Context::bind_group_layout(device)[0]],
            push_constant_ranges: &[],
        });

        let constants = &[("tile_enum_value", PickerValueType::Tile as u32 as f64)];

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants,
                    ..Default::default()
                },
                buffers: &[TileVertex::buffer_layout()],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions {
                    constants,
                    ..Default::default()
                },
                targets: &[Some(ColorTargetState {
                    format: render_pass_context.color_attachment_formats()[0],
                    blend: None,
                    write_mask: ColorWrites::default(),
                })],
            }),
            multiview: None,
            primitive: PrimitiveState::default(),
            multisample: MultisampleState::default(),
            depth_stencil: Some(DepthStencilState {
                format: render_pass_context.depth_attachment_output_format()[0],
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            cache: None,
        });

        Self { pipeline }
    }

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>) {
        if draw_data.index_buffer.count() == 0 {
            return;
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_index_buffer(draw_data.index_buffer.slice(..), IndexFormat::Uint32);
        pass.set_vertex_buffer(0, draw_data.vertex_buffer.slice(..));
        pass.draw_indexed(0..draw_data.index_buffer.count(), 0, 0..1);
    }
}
