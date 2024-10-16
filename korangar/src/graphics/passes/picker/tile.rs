use std::collections::HashMap;

use wgpu::{
    include_wgsl, ColorTargetState, ColorWrites, CompareFunction, DepthStencilState, Device, FragmentState, MultisampleState,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, Queue, RenderPass, RenderPipeline, RenderPipelineDescriptor,
    ShaderModuleDescriptor, VertexState,
};

use crate::graphics::passes::{
    BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, Drawer, PickerRenderPassContext, RenderPassContext,
};
use crate::graphics::picker_target::PickerValueType;
use crate::graphics::{Buffer, GlobalContext, TileVertex};

const SHADER: ShaderModuleDescriptor = include_wgsl!("shader/tile.wgsl");
const DRAWER_NAME: &str = "picker tile";

pub(crate) struct PickerTileDrawer {
    pipeline: RenderPipeline,
}

impl Drawer<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::One }> for PickerTileDrawer {
    type Context = PickerRenderPassContext;
    type DrawData<'draw> = &'draw Buffer<TileVertex>;

    fn new(device: &Device, _queue: &Queue, _global_context: &GlobalContext, render_pass_context: &Self::Context) -> Self {
        let shader_module = device.create_shader_module(SHADER);

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[Self::Context::bind_group_layout(device)[0]],
            push_constant_ranges: &[],
        });

        let mut constants = HashMap::new();
        constants.insert("tile_enum_value".to_owned(), PickerValueType::Tile as u32 as f64);

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some(DRAWER_NAME),
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                compilation_options: PipelineCompilationOptions {
                    constants: &constants,
                    ..Default::default()
                },
                buffers: &[TileVertex::buffer_layout()],
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                compilation_options: PipelineCompilationOptions {
                    constants: &constants,
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
        if draw_data.count() == 0 {
            return;
        }

        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, draw_data.slice(..));
        pass.draw(0..draw_data.count(), 0..1);
    }
}
