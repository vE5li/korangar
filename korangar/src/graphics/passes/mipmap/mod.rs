mod lanczos3;

use std::sync::OnceLock;

pub use lanczos3::Lanczos3Drawer;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource,
    BindingType, Color, CommandEncoder, Device, LoadOp, Operations, RenderPass, RenderPassColorAttachment, RenderPassDescriptor,
    ShaderStages, StoreOp, TextureSampleType, TextureView, TextureViewDimension,
};

const PASS_NAME: &str = "mip map render pass";

#[derive(Default)]
pub struct MipMapRenderPassContext {}

impl MipMapRenderPassContext {
    pub fn create_pass<'encoder>(
        &self,
        device: &Device,
        encoder: &'encoder mut CommandEncoder,
        source_texture: &TextureView,
        destination_texture_view: &TextureView,
    ) -> RenderPass<'encoder> {
        let bind_group = Self::create_bind_group(device, source_texture);

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(PASS_NAME),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: destination_texture_view,
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
        });

        pass.set_bind_group(0, &bind_group, &[]);

        pass
    }

    pub fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; 1] {
        [Self::create_bind_group_layout(device)]
    }

    fn create_bind_group(device: &Device, source_texture_view: &TextureView) -> BindGroup {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(PASS_NAME),
            layout: Self::create_bind_group_layout(device),
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(source_texture_view),
            }],
        })
    }

    fn create_bind_group_layout(device: &Device) -> &'static BindGroupLayout {
        static LAYOUT: OnceLock<BindGroupLayout> = OnceLock::new();
        LAYOUT.get_or_init(|| {
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(PASS_NAME),
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
            })
        })
    }
}
