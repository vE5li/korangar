mod rectangle;

pub(crate) use rectangle::InterfaceRectangleDrawer;
use wgpu::{
    BindGroupLayout, Color, CommandEncoder, Device, LoadOp, Operations, Queue, RenderPass, RenderPassColorAttachment, RenderPassDescriptor,
    StoreOp, TextureFormat,
};

use super::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, RenderPassContext};
use crate::graphics::GlobalContext;
use crate::loaders::TextureLoader;

const PASS_NAME: &str = "interface render pass";

pub(crate) struct InterfaceRenderPassContext {
    interface_texture_format: TextureFormat,
}

impl RenderPassContext<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::None }>
    for InterfaceRenderPassContext
{
    type PassData<'data> = ();

    fn new(_device: &Device, _queue: &Queue, _texture_loader: &TextureLoader, global_context: &GlobalContext) -> Self {
        let interface_texture_format = global_context.interface_buffer_texture.get_format();

        Self { interface_texture_format }
    }

    fn create_pass<'encoder>(
        &mut self,
        encoder: &'encoder mut CommandEncoder,
        global_context: &GlobalContext,
        _: (),
    ) -> RenderPass<'encoder> {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(PASS_NAME),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: global_context.interface_buffer_texture.get_texture_view(),
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_bind_group(0, &global_context.global_bind_group, &[]);

        pass
    }

    fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; 1] {
        [GlobalContext::global_bind_group_layout(device)]
    }

    fn color_attachment_formats(&self) -> [TextureFormat; 1] {
        [self.interface_texture_format]
    }

    fn depth_attachment_output_format(&self) -> [TextureFormat; 0] {
        []
    }
}
