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

#[cfg(test)]
mod test {
    use std::f64::consts::PI;

    fn lanczos3(x: f64) -> f64 {
        match x {
            0.0 => 1.0,
            _ if x.abs() >= 3.0 => 0.0,
            _ => (3.0 * (PI * x).sin() * (PI * x / 3.0).sin()) / (PI * PI * x * x),
        }
    }

    fn generate_lanczos3_kernel() -> Vec<f64> {
        let size = 6;
        let mut kernel = Vec::with_capacity(size * size);
        let mut sum = 0.0;

        // Generate kernel values.
        for y in 0..size {
            for x in 0..size {
                // Center is between pixels 2 and 3 (since the range is 0..=5).
                let dx = (x as f64) - 2.5;
                let dy = (y as f64) - 2.5;
                let value = lanczos3(dx) * lanczos3(dy);
                kernel.push(value);
                sum += value;
            }
        }

        // Normalize the kernel.
        for value in kernel.iter_mut() {
            *value /= sum;
        }

        kernel
    }

    /// This test function can be used to create the lanczos3 kernel parameters.
    #[test]
    fn test_generate_lanczos3_kernel() {
        let kernel = generate_lanczos3_kernel();

        println!("const KERNEL: array<f32, 36> = array<f32, 36>(");
        for chunk in kernel.chunks(6) {
            let line = chunk.iter().map(|x| format!("{:.8}", x)).collect::<Vec<_>>().join(", ");
            println!("    {line},");
        }
        println!(");");
    }
}
