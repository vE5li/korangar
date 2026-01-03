mod entity;
#[cfg(feature = "debug")]
mod marker;
mod tile;

pub(crate) use entity::PickerEntityDrawer;
#[cfg(feature = "debug")]
pub(crate) use marker::PickerMarkerDrawer;
pub(crate) use tile::{PickerTileDrawData, PickerTileDrawer};
use wgpu::{
    BindGroupLayout, CommandEncoder, Device, LoadOp, Operations, Queue, RenderPass, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, TextureFormat,
};

use super::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, RenderPassContext};
use crate::graphics::{GlobalContext, PickerTarget};
use crate::loaders::TextureLoader;

const PASS_NAME: &str = "picker render pass";

pub(crate) struct PickerRenderPassContext {
    picker_texture_format: TextureFormat,
    depth_texture_format: TextureFormat,
}

impl RenderPassContext<{ BindGroupCount::One }, { ColorAttachmentCount::One }, { DepthAttachmentCount::One }> for PickerRenderPassContext {
    type PassData<'data> = Option<()>;

    fn new(_device: &Device, _queue: &Queue, _texture_loader: &TextureLoader, global_context: &GlobalContext) -> Self {
        let picker_texture_format = global_context.picker_buffer_texture.get_format();
        let depth_texture_format = global_context.picker_depth_texture.get_format();

        Self {
            picker_texture_format,
            depth_texture_format,
        }
    }

    fn create_pass<'encoder>(
        &mut self,
        encoder: &'encoder mut CommandEncoder,
        global_context: &GlobalContext,
        _pass_data: Option<()>,
    ) -> RenderPass<'encoder> {
        let (clear_high, clear_low) = <(u32, u32)>::from(PickerTarget::Nothing);
        let clear_color = wgpu::Color {
            r: f64::from(clear_high),
            g: f64::from(clear_low),
            b: 0.0,
            a: 0.0,
        };

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(PASS_NAME),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: global_context.picker_buffer_texture.get_texture_view(),
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(clear_color),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: global_context.picker_depth_texture.get_texture_view(),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.0),
                    store: StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        let unpadded_size = global_context.picker_buffer_texture.get_unpadded_size();
        pass.set_viewport(0.0, 0.0, unpadded_size.width as f32, unpadded_size.height as f32, 0.0, 1.0);
        pass.set_bind_group(0, &global_context.global_bind_group, &[]);

        pass
    }

    fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; 1] {
        [GlobalContext::global_bind_group_layout(device)]
    }

    fn color_attachment_formats(&self) -> [TextureFormat; 1] {
        [self.picker_texture_format]
    }

    fn depth_attachment_output_format(&self) -> [TextureFormat; 1] {
        [self.depth_texture_format]
    }
}
