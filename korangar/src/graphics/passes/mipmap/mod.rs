mod lanczos3;

use std::num::NonZeroU64;
use std::sync::OnceLock;

use bytemuck::{Pod, Zeroable};
pub use lanczos3::Lanczos3Drawer;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, BufferBindingType, BufferUsages, Color, CommandEncoder, Device, LoadOp, Operations, Queue, RenderPass,
    RenderPassColorAttachment, RenderPassDescriptor, ShaderStages, StoreOp, TextureSampleType, TextureView, TextureViewDimension,
};

use crate::graphics::Buffer;

const PASS_NAME: &str = "mip map render pass";

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub(crate) struct MipMapUniforms {
    // Lanczos3 6x6 resampling kernel (packed as vec4 for std140 layout)
    lanczos3_kernel: [[f32; 4]; 9],
}

impl MipMapUniforms {
    const fn lanczos3(x: f32) -> f32 {
        const LANCZOS3_2_5: f32 = 0.024317075;
        const LANCZOS3_1_5: f32 = -0.1350949;
        const LANCZOS3_0_5: f32 = 0.6079271;

        match x {
            0.0 => 1.0,
            -2.5 | 2.5 => LANCZOS3_2_5,
            -1.5 | 1.5 => LANCZOS3_1_5,
            -0.5 | 0.5 => LANCZOS3_0_5,
            _ if x.abs() >= 3.0 => 0.0,
            _ => panic!(
                "Unknown Lanczos3 x value. Please pre-compute the value using:
            (3.0 * (PI * x).sin() * (PI * x / 3.0).sin()) / (PI * PI * x * x)"
            ),
        }
    }

    const fn generate_lanczos3_kernel() -> [f32; 36] {
        const SIZE: usize = 6;
        let mut kernel = [0f32; SIZE * SIZE];
        let mut sum = 0.0;

        let mut y = 0;
        while y < SIZE {
            let mut x = 0;
            while x < SIZE {
                let dx = (x as f32) - 2.5;
                let dy = (y as f32) - 2.5;
                let value = Self::lanczos3(dx) * Self::lanczos3(dy);
                kernel[y * SIZE + x] = value;
                sum += value;
                x += 1;
            }
            y += 1;
        }

        // Normalize the kernel.
        let mut i = 0;
        while i < kernel.len() {
            kernel[i] /= sum;
            i += 1;
        }

        kernel
    }

    const fn initialize() -> Self {
        let kernel = Self::generate_lanczos3_kernel();
        let mut packed_kernel = [[0.0f32; 4]; 9];

        let mut i = 0;
        while i < 9 {
            packed_kernel[i] = [kernel[i * 4], kernel[i * 4 + 1], kernel[i * 4 + 2], kernel[i * 4 + 3]];
            i += 1;
        }

        Self {
            lanczos3_kernel: packed_kernel,
        }
    }
}

pub struct MipMapRenderPassContext {
    mipmap_uniforms_buffer: Buffer<MipMapUniforms>,
}

impl MipMapRenderPassContext {
    pub fn new(device: &Device, queue: &Queue) -> Self {
        let mipmap_uniforms_buffer = Buffer::with_capacity(
            device,
            "mipmap uniforms",
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            size_of::<MipMapUniforms>() as _,
        );
        // The data is static and only needs to be uploaded once.
        mipmap_uniforms_buffer.write_exact(queue, &[MipMapUniforms::initialize()]);

        Self { mipmap_uniforms_buffer }
    }

    pub fn create_pass<'encoder>(
        &self,
        device: &Device,
        encoder: &'encoder mut CommandEncoder,
        source_texture: &TextureView,
        destination_texture_view: &TextureView,
    ) -> RenderPass<'encoder> {
        let bind_group = Self::create_bind_group(device, &self.mipmap_uniforms_buffer, source_texture);

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(PASS_NAME),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: destination_texture_view,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 1.0,
                        g: 0.0,
                        b: 1.0,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_bind_group(0, &bind_group, &[]);

        pass
    }

    pub fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; 1] {
        [Self::create_bind_group_layout(device)]
    }

    fn create_bind_group(device: &Device, mipmap_uniforms_buffer: &Buffer<MipMapUniforms>, source_texture_view: &TextureView) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(PASS_NAME),
            layout: Self::create_bind_group_layout(device),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: mipmap_uniforms_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(source_texture_view),
                },
            ],
        })
    }

    fn create_bind_group_layout(device: &Device) -> &'static BindGroupLayout {
        static LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(PASS_NAME),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(size_of::<MipMapUniforms>() as _),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            })
        })
    }
}
