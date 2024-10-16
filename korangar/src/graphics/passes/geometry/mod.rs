mod entity;
mod indicator;
mod model;
mod water;

pub(crate) use entity::GeometryEntityDrawer;
pub(crate) use indicator::GeometryIndicatorDrawer;
pub(crate) use model::GeometryModelDrawer;
pub(crate) use water::GeometryWaterDrawer;
use wgpu::{
    BindGroupLayout, Color, CommandEncoder, Device, LoadOp, Operations, Queue, RenderPass, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, TextureFormat, TextureView,
};

use super::{BindGroupCount, ColorAttachmentCount, DepthAttachmentCount, RenderPassContext};
use crate::graphics::GlobalContext;
use crate::loaders::TextureLoader;
const PASS_NAME: &str = "geometry render pass";

pub(crate) struct GeometryRenderPassContext {
    diffuse_buffer_texture_format: TextureFormat,
    normal_buffer_texture_format: TextureFormat,
    water_buffer_texture_format: TextureFormat,
    depth_buffer_texture_format: TextureFormat,
}

impl RenderPassContext<{ BindGroupCount::One }, { ColorAttachmentCount::Three }, { DepthAttachmentCount::One }>
    for GeometryRenderPassContext
{
    type PassData<'data> = Option<()>;

    fn new(_device: &Device, _queue: &Queue, _texture_loader: &TextureLoader, global_context: &GlobalContext) -> Self {
        let diffuse_buffer_texture_format = global_context.diffuse_buffer_texture.get_format();
        let normal_buffer_texture_format = global_context.normal_buffer_texture.get_format();
        let water_buffer_texture_format = global_context.water_buffer_texture.get_format();
        let depth_buffer_texture_format = global_context.depth_buffer_texture.get_format();

        Self {
            diffuse_buffer_texture_format,
            normal_buffer_texture_format,
            water_buffer_texture_format,
            depth_buffer_texture_format,
        }
    }

    fn create_pass<'encoder>(
        &mut self,
        _frame_view: &TextureView,
        encoder: &'encoder mut CommandEncoder,
        global_context: &GlobalContext,
        _pass_data: Option<()>,
    ) -> RenderPass<'encoder> {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some(PASS_NAME),
            color_attachments: &[
                Some(RenderPassColorAttachment {
                    view: global_context.diffuse_buffer_texture.get_texture_view(),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                }),
                Some(RenderPassColorAttachment {
                    view: global_context.normal_buffer_texture.get_texture_view(),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                }),
                Some(RenderPassColorAttachment {
                    view: global_context.water_buffer_texture.get_texture_view(),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::BLACK),
                        store: StoreOp::Store,
                    },
                }),
            ],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: global_context.depth_buffer_texture.get_texture_view(),
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

        pass
    }

    fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; 1] {
        [GlobalContext::global_bind_group_layout(device)]
    }

    fn color_attachment_formats(&self) -> [TextureFormat; 3] {
        [
            self.diffuse_buffer_texture_format,
            self.normal_buffer_texture_format,
            self.water_buffer_texture_format,
        ]
    }

    fn depth_attachment_output_format(&self) -> [TextureFormat; 1] {
        [self.depth_buffer_texture_format]
    }
}
