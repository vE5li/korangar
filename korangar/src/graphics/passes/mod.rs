mod directional_shadow;
mod forward;
mod interface;
mod light_culling;
mod mipmap;
mod picker;
mod point_shadow;
mod postprocessing;
mod screen_blit;
use std::marker::ConstParamTy;

use bytemuck::{Pod, Zeroable};
pub(crate) use directional_shadow::*;
pub(crate) use forward::*;
pub(crate) use interface::*;
pub(crate) use light_culling::*;
pub use mipmap::*;
pub(crate) use picker::*;
pub(crate) use point_shadow::*;
pub(crate) use postprocessing::*;
pub(crate) use screen_blit::*;
use wgpu::{BindGroupLayout, CommandEncoder, ComputePass, Device, Queue, RenderPass, TextureFormat};

use crate::graphics::{Capabilities, GlobalContext, ModelBatch, ModelInstruction};
use crate::loaders::TextureLoader;

#[derive(Clone, Copy, PartialEq, Eq, ConstParamTy)]
pub(crate) enum BindGroupCount {
    None = 0,
    One = 1,
    Two = 2,
}

#[derive(Clone, Copy, PartialEq, Eq, ConstParamTy)]
pub(crate) enum ColorAttachmentCount {
    None = 0,
    One = 1,
    Three = 3,
}

#[derive(Clone, Copy, PartialEq, Eq, ConstParamTy)]
pub(crate) enum DepthAttachmentCount {
    None = 0,
    One = 1,
}

/// Gives render passes the context they need to execute. They are the owner of
/// the resources that are pass specific and shared by multiple drawer.
#[allow(unused)]
pub(crate) trait RenderPassContext<const BIND: BindGroupCount, const COLOR: ColorAttachmentCount, const DEPTH: DepthAttachmentCount> {
    type PassData<'data>;

    /// Creates a new render pass context.
    fn new(device: &Device, queue: &Queue, texture_loader: &TextureLoader, global_context: &GlobalContext) -> Self;

    /// Crates a render new pass.
    fn create_pass<'encoder>(
        &mut self,
        encoder: &'encoder mut CommandEncoder,
        global_context: &GlobalContext,
        pass_data: Self::PassData<'_>,
    ) -> RenderPass<'encoder>;

    /// The bind group layout of the render pass.
    fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; BIND as usize];

    /// The formats of all color attachments that this pass sets.
    fn color_attachment_formats(&self) -> [TextureFormat; COLOR as usize];

    /// The formats of all depth attachments that this pass sets.
    fn depth_attachment_output_format(&self) -> [TextureFormat; DEPTH as usize];
}

/// Gives compute passes the context they need to execute. They are the owner of
/// that resources that are pass specific and shared by multiple drawer.
#[allow(unused)]
pub(crate) trait ComputePassContext<const BIND: BindGroupCount> {
    type PassData<'data>;

    /// Creates a new compute pass context.
    fn new(device: &Device, queue: &Queue, global_context: &GlobalContext) -> Self;

    /// Crates a compute new pass.
    fn create_pass<'encoder>(
        &mut self,
        encoder: &'encoder mut CommandEncoder,
        global_context: &GlobalContext,
        pass_data: Self::PassData<'_>,
    ) -> ComputePass<'encoder>;

    /// The bind group layout of the compute pass.
    fn bind_group_layout(device: &Device) -> [&'static BindGroupLayout; BIND as usize];
}

/// Trait for structures that do draw operations inside a render pass.
#[allow(unused)]
pub(crate) trait Drawer<const BIND: BindGroupCount, const COLOR: ColorAttachmentCount, const DEPTH: DepthAttachmentCount> {
    type Context: RenderPassContext<BIND, COLOR, DEPTH>;
    type DrawData<'data>;

    fn new(
        capabilities: &Capabilities,
        device: &Device,
        queue: &Queue,
        global_context: &GlobalContext,
        render_pass_context: &Self::Context,
    ) -> Self;

    fn draw(&mut self, pass: &mut RenderPass<'_>, draw_data: Self::DrawData<'_>);
}

/// Trait for structures that do dispatch operations inside a compute pass.
#[allow(unused)]
pub(crate) trait Dispatch<const BIND: BindGroupCount> {
    type Context: ComputePassContext<BIND>;
    type DispatchData<'data>;

    fn new(
        capabilities: &Capabilities,
        device: &Device,
        queue: &Queue,
        global_context: &GlobalContext,
        compute_pass_context: &Self::Context,
    ) -> Self;

    fn dispatch(&mut self, pass: &mut ComputePass<'_>, draw_data: Self::DispatchData<'_>);
}

/// We reimplement the WGPU type, since we want to have bytemuck support.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub(crate) struct DrawIndexedIndirectArgs {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
}

/// A batch of models that share a specific texture group and model vertex
/// buffer.
pub(crate) struct ModelBatchDrawData<'a> {
    pub(crate) batches: &'a [ModelBatch],
    pub(crate) instructions: &'a [ModelInstruction],
    #[cfg(feature = "debug")]
    pub(crate) show_wireframe: bool,
}
