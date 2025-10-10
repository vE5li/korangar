use wgpu::{
    ColorTargetState, ColorWrites, Device, FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, RenderPass, RenderPipeline, RenderPipelineDescriptor, TextureFormat, VertexState,
};

use crate::graphics::passes::mipmap::MipMapRenderPassContext;
use crate::graphics::shader_compiler::ShaderCompiler;

const DRAWER_NAME: &str = "lanczos3";

pub struct Lanczos3Drawer {
    pipeline: RenderPipeline,
}

impl Lanczos3Drawer {
    pub fn new(device: &Device, shader_compiler: &ShaderCompiler) -> Self {
        let shader_module = shader_compiler.create_shader_module("mipmap", "lanczos3");

        let pass_bind_group_layouts = MipMapRenderPassContext::bind_group_layout(device);

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some(DRAWER_NAME),
            bind_group_layouts: &[pass_bind_group_layouts[0]],
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
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba8UnormSrgb,
                    blend: None,
                    write_mask: ColorWrites::default(),
                })],
            }),
            multiview: None,
            cache: None,
        });

        Self { pipeline }
    }

    pub fn draw(&self, pass: &mut RenderPass<'_>) {
        pass.set_pipeline(&self.pipeline);
        pass.draw(0..3, 0..1);
    }
}
