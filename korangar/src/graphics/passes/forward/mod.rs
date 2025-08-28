mod entity;
mod indicator;
mod model;
mod wave;

pub(crate) use entity::{EntityPassMode, ForwardEntityDrawData, ForwardEntityDrawer};
pub(crate) use indicator::ForwardIndicatorDrawer;
pub(crate) use model::{ForwardModelDrawData, ForwardModelDrawer, ModelPassMode};
pub(crate) use wave::WaterWaveDrawer;
use wgpu::{
    BindGroupLayout, Color, CommandEncoder, Device, LoadOp, Operations, Queue, RenderPass, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, TextureFormat,
};

use super::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, RenderPassContext};
use crate::graphics::GlobalContext;
use crate::loaders::TextureLoader;
const PASS_NAME: &str = "forward render pass";

pub(crate) struct ForwardRenderPassContext {
    color_texture_format: TextureFormat,
    accumulation_texture_format: TextureFormat,
    revealage_texture_format: TextureFormat,
    depth_texture_format: TextureFormat,
}

impl RenderPassContext<{ BindGroupCount::Two }, { ColorAttachmentCount::Three }, { DepthAttachmentCount::One }>
    for ForwardRenderPassContext
{
    type PassData<'data> = Option<()>;

    fn new(_device: &Device, _queue: &Queue, _texture_loader: &TextureLoader, global_context: &GlobalContext) -> Self {
        let color_texture_format = global_context.forward_color_texture.get_format();
        let accumulation_texture_format = global_context.forward_accumulation_texture.get_format();
        let revealage_texture_format = global_context.forward_revealage_texture.get_format();
        let depth_texture_format = global_context.forward_depth_texture.get_format();

        Self {
            color_texture_format,
            accumulation_texture_format,
            revealage_texture_format,
            depth_texture_format,
        }
    }

    fn create_pass<'encoder>(
        &mut self,
        encoder: &'encoder mut CommandEncoder,
        global_context: &GlobalContext,
        _pass_data: Option<()>,
    ) -> RenderPass<'encoder> {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(PASS_NAME),
            color_attachments: &[
                Some(RenderPassColorAttachment {
                    view: global_context.forward_color_texture.get_texture_view(),
                    depth_slice: None,
                    resolve_target: global_context
                        .resolved_color_texture
                        .as_ref()
                        .map(|texture| texture.get_texture_view()),
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                }),
                Some(RenderPassColorAttachment {
                    view: global_context.forward_accumulation_texture.get_texture_view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                }),
                Some(RenderPassColorAttachment {
                    view: global_context.forward_revealage_texture.get_texture_view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::WHITE),
                        store: StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: global_context.forward_depth_texture.get_texture_view(),
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(0.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_bind_group(0, &global_context.global_bind_group, &[]);
        pass.set_bind_group(1, &global_context.forward_bind_group, &[]);

        pass
    }

    fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; 2] {
        [
            GlobalContext::global_bind_group_layout(device),
            GlobalContext::forward_bind_group_layout(device),
        ]
    }

    fn color_attachment_formats(&self) -> [TextureFormat; 3] {
        [
            self.color_texture_format,
            self.accumulation_texture_format,
            self.revealage_texture_format,
        ]
    }

    fn depth_attachment_output_format(&self) -> [TextureFormat; 1] {
        [self.depth_texture_format]
    }
}
