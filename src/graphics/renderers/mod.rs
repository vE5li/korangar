mod deferred;
mod interface;
mod picker;
#[cfg(feature = "debug")]
mod settings;
mod shadow;
mod swapchain;

use std::marker::PhantomData;
use std::sync::Arc;

use cgmath::{Matrix4, Vector2, Vector3};
use option_ext::OptionExt;
use procedural::profile;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, ClearAttachment, ClearRect, CommandBufferUsage, CopyImageToBufferInfo, PrimaryAutoCommandBuffer,
    PrimaryCommandBufferAbstract, RenderPassBeginInfo, SubpassContents,
};
use vulkano::device::Queue;
use vulkano::format::{ClearColorValue, ClearValue, Format};
use vulkano::image::view::ImageView;
use vulkano::image::{AttachmentImage, ImageUsage, SampleCount, SwapchainImage};
use vulkano::pipeline::graphics::color_blend::{AttachmentBlend, BlendFactor, BlendOp};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass};
use vulkano::swapchain::{Swapchain, SwapchainPresentInfo};
use vulkano::sync::{FenceSignalFuture, GpuFuture, SemaphoreSignalFuture};

pub use self::deferred::DeferredRenderer;
use self::deferred::DeferredSubrenderer;
pub use self::interface::InterfaceRenderer;
pub use self::picker::PickerRenderer;
use self::picker::PickerSubrenderer;
#[cfg(feature = "debug")]
pub use self::settings::RenderSettings;
pub use self::shadow::ShadowRenderer;
pub use self::swapchain::{PresentModeInfo, SwapchainHolder};
use super::{Color, MemoryAllocator};
#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::{Camera, ImageBuffer, ModelVertexBuffer, Texture};
use crate::network::EntityId;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;
use crate::world::Tile;

pub const LIGHT_ATTACHMENT_BLEND: AttachmentBlend = AttachmentBlend {
    color_op: BlendOp::Add,
    color_source: BlendFactor::One,
    color_destination: BlendFactor::One,
    alpha_op: BlendOp::Max,
    alpha_source: BlendFactor::One,
    alpha_destination: BlendFactor::One,
};

pub const WATER_ATTACHMENT_BLEND: AttachmentBlend = AttachmentBlend {
    color_op: BlendOp::ReverseSubtract,
    color_source: BlendFactor::One,
    color_destination: BlendFactor::One,
    alpha_op: BlendOp::Max,
    alpha_source: BlendFactor::One,
    alpha_destination: BlendFactor::One,
};

pub const INTERFACE_ATTACHMENT_BLEND: AttachmentBlend = AttachmentBlend {
    color_op: BlendOp::Add,
    color_source: BlendFactor::SrcAlpha,
    color_destination: BlendFactor::OneMinusSrcAlpha,
    alpha_op: BlendOp::Max,
    alpha_source: BlendFactor::SrcAlpha,
    alpha_destination: BlendFactor::DstAlpha,
};

pub trait Renderer {
    type Target;
}

pub trait GeometryRenderer {
    fn render_geometry(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        vertex_buffer: ModelVertexBuffer,
        textures: &[Texture],
        world_matrix: Matrix4<f32>,
        time: f32,
    ) where
        Self: Renderer;
}

pub trait EntityRenderer {
    fn render_entity(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        texture: Texture,
        position: Vector3<f32>,
        origin: Vector3<f32>,
        scale: Vector2<f32>,
        cell_count: Vector2<usize>,
        cell_position: Vector2<usize>,
        mirror: bool,
        entity_id: EntityId,
    ) where
        Self: Renderer;
}

pub trait IndicatorRenderer {
    fn render_walk_indicator(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        color: Color,
        upper_left: Vector3<f32>,
        upper_right: Vector3<f32>,
        lower_left: Vector3<f32>,
        lower_right: Vector3<f32>,
    ) where
        Self: Renderer;
}

#[derive(Debug)]
pub enum PickerTarget {
    Tile {
        x: u16,
        y: u16,
    },
    Entity(EntityId),
    #[cfg(feature = "debug")]
    Marker(MarkerIdentifier),
}

impl From<u32> for PickerTarget {
    fn from(data: u32) -> Self {
        if data >> 31 == 1 {
            let x = ((data >> 16) as u16) ^ (1 << 15);
            let y = data as u16;
            return Self::Tile { x, y };
        }

        #[cfg(feature = "debug")]
        if data >> 24 == 10 {
            return Self::Marker(MarkerIdentifier::Object(data as usize & 0xfff));
        }

        #[cfg(feature = "debug")]
        if data >> 24 == 11 {
            return Self::Marker(MarkerIdentifier::LightSource(data as usize & 0xfff));
        }

        #[cfg(feature = "debug")]
        if data >> 24 == 12 {
            return Self::Marker(MarkerIdentifier::SoundSource(data as usize & 0xfff));
        }

        #[cfg(feature = "debug")]
        if data >> 24 == 13 {
            return Self::Marker(MarkerIdentifier::EffectSource(data as usize & 0xfff));
        }

        #[cfg(feature = "debug")]
        if data >> 24 == 14 {
            return Self::Marker(MarkerIdentifier::Entity(data as usize & 0xfff));
        }

        let entity_id = match data >> 24 == 5 {
            true => data ^ (5 << 24),
            false => data,
        };

        Self::Entity(EntityId(entity_id))
    }
}

impl From<PickerTarget> for u32 {
    fn from(picker_target: PickerTarget) -> Self {
        match picker_target {
            PickerTarget::Tile { x, y } => {
                let mut encoded = ((x as u32) << 16) | y as u32;
                encoded |= 1 << 31;
                encoded
            }
            PickerTarget::Entity(EntityId(entity_id)) => match entity_id >> 24 == 0 {
                true => entity_id | (5 << 24),
                false => entity_id,
            },
            #[cfg(feature = "debug")]
            PickerTarget::Marker(marker_identifier) => match marker_identifier {
                MarkerIdentifier::Object(index) => (10 << 24) | (index as u32 & 0xfff),
                MarkerIdentifier::LightSource(index) => (11 << 24) | (index as u32 & 0xfff),
                MarkerIdentifier::SoundSource(index) => (12 << 24) | (index as u32 & 0xfff),
                MarkerIdentifier::EffectSource(index) => (13 << 24) | (index as u32 & 0xfff),
                MarkerIdentifier::Entity(index) => (14 << 24) | (index as u32 & 0xfff),
                _ => panic!(),
            },
        }
    }
}

#[cfg(feature = "debug")]
pub trait MarkerRenderer {
    fn render_marker(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        position: Vector3<f32>,
        hovered: bool,
    ) where
        Self: Renderer;
}

pub enum RenderTargetState {
    Ready,
    Rendering(AutoCommandBufferBuilder<PrimaryAutoCommandBuffer, MemoryAllocator>),
    Semaphore(SemaphoreSignalFuture<Box<dyn GpuFuture>>),
    Fence(FenceSignalFuture<Box<dyn GpuFuture>>),
    OutOfDate,
}

unsafe impl Send for RenderTargetState {}

impl RenderTargetState {
    pub fn get_builder(&mut self) -> &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer, MemoryAllocator> {
        let RenderTargetState::Rendering(builder) = self else {
            panic!("render target is not in the render state");
        };

        builder
    }

    pub fn take_builder(&mut self) -> AutoCommandBufferBuilder<PrimaryAutoCommandBuffer, MemoryAllocator> {
        let RenderTargetState::Rendering(builder) = std::mem::replace(self, RenderTargetState::Ready) else {
            panic!("render target is not in the render state");
        };

        builder
    }

    pub fn take_semaphore(&mut self) -> SemaphoreSignalFuture<Box<dyn GpuFuture>> {
        let RenderTargetState::Semaphore(semaphore) = std::mem::replace(self, RenderTargetState::Ready) else {
            panic!("render target is not in the semaphore state");
        };

        semaphore
    }

    pub fn try_take_semaphore(&mut self) -> Option<Box<dyn GpuFuture>> {
        if let RenderTargetState::Ready = self {
            return None;
        }

        let RenderTargetState::Semaphore(semaphore) = std::mem::replace(self, RenderTargetState::Ready) else {
            panic!("render target is in an unexpected state");
        };

        semaphore.boxed().into()
    }

    pub fn try_take_fence(&mut self) -> Option<FenceSignalFuture<Box<dyn GpuFuture>>> {
        if let RenderTargetState::Ready = self {
            return None;
        }

        let RenderTargetState::Fence(fence) = std::mem::replace(self, RenderTargetState::Ready) else {
            panic!("render target is in an unexpected state");
        };

        fence.into()
    }
}

pub struct DeferredRenderTarget {
    memory_allocator: Arc<MemoryAllocator>,
    queue: Arc<Queue>,
    framebuffer: Arc<Framebuffer>,
    diffuse_image: ImageBuffer,
    normal_image: ImageBuffer,
    water_image: ImageBuffer,
    depth_image: ImageBuffer,
    pub state: RenderTargetState,
    bound_subrenderer: Option<DeferredSubrenderer>,
}

impl DeferredRenderTarget {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        queue: Arc<Queue>,
        render_pass: Arc<RenderPass>,
        swapchain_image: Arc<SwapchainImage>,
        dimensions: [u32; 2],
    ) -> Self {
        let color_image_usage = ImageUsage {
            sampled: true,
            color_attachment: true,
            input_attachment: true,
            ..ImageUsage::empty()
        };

        let depth_image_usage = ImageUsage {
            sampled: true,
            depth_stencil_attachment: true,
            input_attachment: true,
            ..ImageUsage::empty()
        };

        let diffuse_image = ImageView::new_default(Arc::new(
            AttachmentImage::multisampled_with_usage(
                &*memory_allocator,
                dimensions,
                SampleCount::Sample4,
                Format::R32G32B32A32_SFLOAT,
                color_image_usage,
            )
            .unwrap(),
        ))
        .unwrap();

        let normal_image = ImageView::new_default(Arc::new(
            AttachmentImage::multisampled_with_usage(
                &*memory_allocator,
                dimensions,
                SampleCount::Sample4,
                Format::R16G16B16A16_SFLOAT,
                color_image_usage,
            )
            .unwrap(),
        ))
        .unwrap();

        let water_image = ImageView::new_default(Arc::new(
            AttachmentImage::multisampled_with_usage(
                &*memory_allocator,
                dimensions,
                SampleCount::Sample4,
                Format::R8G8B8A8_UNORM,
                color_image_usage,
            )
            .unwrap(),
        ))
        .unwrap();

        let depth_image = ImageView::new_default(Arc::new(
            AttachmentImage::multisampled_with_usage(
                &*memory_allocator,
                dimensions,
                SampleCount::Sample4,
                Format::D32_SFLOAT,
                depth_image_usage,
            )
            .unwrap(),
        ))
        .unwrap();

        let framebuffer_create_info = FramebufferCreateInfo {
            attachments: vec![
                ImageView::new_default(swapchain_image).unwrap(),
                diffuse_image.clone(),
                normal_image.clone(),
                water_image.clone(),
                depth_image.clone(),
            ],
            ..Default::default()
        };

        let framebuffer = Framebuffer::new(render_pass, framebuffer_create_info).unwrap();
        let state = RenderTargetState::Ready;
        let bound_subrenderer = None;

        Self {
            memory_allocator,
            queue,
            framebuffer,
            diffuse_image,
            normal_image,
            water_image,
            depth_image,
            state,
            bound_subrenderer,
        }
    }

    #[profile("start frame")]
    pub fn start(&mut self) {
        let mut builder = AutoCommandBufferBuilder::primary(
            &*self.memory_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let render_pass_begin_info = RenderPassBeginInfo {
            clear_values: vec![
                Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
                Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
                Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
                Some(ClearValue::Float([0.0, 0.0, 0.0, 1.0])),
                Some(ClearValue::Depth(1.0)),
            ],
            ..RenderPassBeginInfo::framebuffer(self.framebuffer.clone())
        };

        builder.begin_render_pass(render_pass_begin_info, SubpassContents::Inline).unwrap();

        self.state = RenderTargetState::Rendering(builder);
    }

    pub fn bind_subrenderer(&mut self, subrenderer: DeferredSubrenderer) -> bool {
        let already_bound = self.bound_subrenderer.contains(&subrenderer);
        self.bound_subrenderer = Some(subrenderer);
        !already_bound
    }

    pub fn unbind_subrenderer(&mut self) {
        self.bound_subrenderer = None;
    }

    pub fn lighting_pass(&mut self) {
        self.state.get_builder().next_subpass(SubpassContents::Inline).unwrap();
    }

    #[profile("finish swapchain image")]
    pub fn finish(&mut self, swapchain: Arc<Swapchain>, semaphore: Box<dyn GpuFuture>, image_number: usize) {
        let mut builder = self.state.take_builder();

        #[cfg(feature = "debug")]
        let end_render_pass_measurement = start_measurement("end render pass");

        builder.end_render_pass().unwrap();

        #[cfg(feature = "debug")]
        end_render_pass_measurement.stop();

        let command_buffer = builder.build().unwrap();

        #[cfg(feature = "debug")]
        let swapchain_measurement = start_measurement("get next swapchain image");

        // TODO: make this type ImageNumber instead
        let present_info = SwapchainPresentInfo::swapchain_image_index(swapchain, image_number as u32);

        #[cfg(feature = "debug")]
        swapchain_measurement.stop();

        #[cfg(feature = "debug")]
        let execute_measurement = start_measurement("queue command buffer");

        let future = semaphore.then_execute(self.queue.clone(), command_buffer).unwrap();

        #[cfg(feature = "debug")]
        execute_measurement.stop();

        #[cfg(feature = "debug")]
        let present_measurement = start_measurement("present swapchain");

        let future = future.then_swapchain_present(self.queue.clone(), present_info).boxed();

        #[cfg(feature = "debug")]
        present_measurement.stop();

        #[cfg(feature = "debug")]
        let flush_measurement = start_measurement("flush");

        self.state = future
            .then_signal_fence_and_flush()
            .map(RenderTargetState::Fence)
            .unwrap_or(RenderTargetState::OutOfDate);

        #[cfg(feature = "debug")]
        flush_measurement.stop();

        self.bound_subrenderer = None;
    }
}

pub struct PickerRenderTarget {
    memory_allocator: Arc<MemoryAllocator>,
    queue: Arc<Queue>,
    framebuffer: Arc<Framebuffer>,
    pub image: ImageBuffer,
    pub buffer: Arc<CpuAccessibleBuffer<[u32]>>,
    pub state: RenderTargetState,
    bound_subrenderer: Option<PickerSubrenderer>,
}

impl PickerRenderTarget {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, queue: Arc<Queue>, render_pass: Arc<RenderPass>, dimensions: [u32; 2]) -> Self {
        let image_usage = ImageUsage {
            sampled: true,
            transfer_src: true,
            color_attachment: true,
            ..ImageUsage::empty()
        };

        let depth_image_usage = ImageUsage {
            depth_stencil_attachment: true,
            ..ImageUsage::empty()
        };

        let image = ImageView::new_default(Arc::new(
            AttachmentImage::with_usage(&*memory_allocator, dimensions, Format::R32_UINT, image_usage).unwrap(),
        ))
        .unwrap();

        let depth_buffer = ImageView::new_default(Arc::new(
            AttachmentImage::with_usage(&*memory_allocator, dimensions, Format::D16_UNORM, depth_image_usage).unwrap(),
        ))
        .unwrap();

        let framebuffer_create_info = FramebufferCreateInfo {
            attachments: vec![image.clone(), depth_buffer],
            ..Default::default()
        };

        let framebuffer = Framebuffer::new(render_pass, framebuffer_create_info).unwrap();

        let buffer = unsafe {
            CpuAccessibleBuffer::uninitialized_array(
                &*memory_allocator,
                dimensions[0] as u64 * dimensions[1] as u64,
                BufferUsage {
                    transfer_dst: true,
                    ..Default::default()
                },
                false,
            )
            .unwrap()
        };
        let state = RenderTargetState::Ready;
        let bound_subrenderer = None;

        Self {
            memory_allocator,
            queue,
            framebuffer,
            image,
            buffer,
            state,
            bound_subrenderer,
        }
    }

    #[profile("start frame")]
    pub fn start(&mut self) {
        let mut builder = AutoCommandBufferBuilder::primary(
            &*self.memory_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let render_pass_begin_info = RenderPassBeginInfo {
            clear_values: vec![Some(ClearValue::Uint([0; 4])), Some(ClearValue::Depth(1.0))],
            ..RenderPassBeginInfo::framebuffer(self.framebuffer.clone())
        };

        builder.begin_render_pass(render_pass_begin_info, SubpassContents::Inline).unwrap();

        self.state = RenderTargetState::Rendering(builder);
    }

    #[profile]
    pub fn bind_subrenderer(&mut self, subrenderer: PickerSubrenderer) -> bool {
        let already_bound = self.bound_subrenderer.contains(&subrenderer);
        self.bound_subrenderer = Some(subrenderer);
        !already_bound
    }

    #[profile]
    pub fn unbind_subrenderer(&mut self) {
        self.bound_subrenderer = None;
    }

    #[profile("finish buffer")]
    pub fn finish(&mut self) {
        let mut builder = self.state.take_builder();

        builder.end_render_pass().unwrap();
        builder
            .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(
                self.image.image().clone(),
                self.buffer.clone(),
            ))
            .unwrap();

        let command_buffer = builder.build().unwrap();
        let fence = command_buffer
            .execute(self.queue.clone())
            .unwrap()
            .boxed()
            .then_signal_fence_and_flush()
            .unwrap();

        self.state = RenderTargetState::Fence(fence);
        self.bound_subrenderer = None;
    }
}

pub trait IntoFormat {
    fn into_format() -> Format;
}

pub struct SingleRenderTarget<F: IntoFormat, S: PartialEq, C> {
    memory_allocator: Arc<MemoryAllocator>,
    queue: Arc<Queue>,
    framebuffer: Arc<Framebuffer>,
    pub image: ImageBuffer,
    pub state: RenderTargetState,
    clear_value: C,
    bound_subrenderer: Option<S>,
    _phantom_data: PhantomData<F>,
}

impl<F: IntoFormat, S: PartialEq, C> SingleRenderTarget<F, S, C> {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        queue: Arc<Queue>,
        render_pass: Arc<RenderPass>,
        dimensions: [u32; 2],
        sample_count: SampleCount,
        image_usage: ImageUsage,
        clear_value: C,
    ) -> Self {
        let image = ImageView::new_default(Arc::new(
            AttachmentImage::multisampled_with_usage(&*memory_allocator, dimensions, sample_count, F::into_format(), image_usage).unwrap(),
        ))
        .unwrap();

        let framebuffer_create_info = FramebufferCreateInfo {
            attachments: vec![image.clone()],
            ..Default::default()
        };

        let framebuffer = Framebuffer::new(render_pass, framebuffer_create_info).unwrap();

        let state = RenderTargetState::Ready;
        let bound_subrenderer = None;

        Self {
            memory_allocator,
            queue,
            framebuffer,
            image,
            state,
            clear_value,
            bound_subrenderer,
            _phantom_data: Default::default(),
        }
    }

    #[profile]
    pub fn unbind_subrenderer(&mut self) {
        self.bound_subrenderer = None;
    }

    #[profile]
    pub fn bind_subrenderer(&mut self, subrenderer: S) -> bool {
        let already_bound = self.bound_subrenderer.contains(&subrenderer);
        self.bound_subrenderer = Some(subrenderer);
        !already_bound
    }
}

impl<F: IntoFormat, S: PartialEq> SingleRenderTarget<F, S, ClearValue> {
    #[profile("start frame")]
    pub fn start(&mut self) {
        let mut builder = AutoCommandBufferBuilder::primary(
            &*self.memory_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let render_pass_begin_info = RenderPassBeginInfo {
            clear_values: vec![Some(self.clear_value)],
            ..RenderPassBeginInfo::framebuffer(self.framebuffer.clone())
        };

        builder.begin_render_pass(render_pass_begin_info, SubpassContents::Inline).unwrap();
        self.state = RenderTargetState::Rendering(builder);
    }

    #[profile("finalize buffer")]
    pub fn finish(&mut self) {
        let mut builder = self.state.take_builder();

        builder.end_render_pass().unwrap();

        let command_buffer = builder.build().unwrap();
        let semaphore = command_buffer
            .execute(self.queue.clone())
            .unwrap()
            .boxed()
            .then_signal_semaphore_and_flush()
            .unwrap();

        self.state = RenderTargetState::Semaphore(semaphore);
        self.bound_subrenderer = None;
    }
}

impl<F: IntoFormat, S: PartialEq> SingleRenderTarget<F, S, ClearColorValue> {
    #[profile("start frame")]
    pub fn start(&mut self, dimensions: [u32; 2], clear_interface: bool) {
        // TODO:

        let mut builder = AutoCommandBufferBuilder::primary(
            &*self.memory_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let render_pass_begin_info = RenderPassBeginInfo {
            clear_values: vec![None],
            ..RenderPassBeginInfo::framebuffer(self.framebuffer.clone())
        };

        builder.begin_render_pass(render_pass_begin_info, SubpassContents::Inline).unwrap();

        if clear_interface {
            /*let image: Arc<dyn ImageAccess> = Arc::clone(&self.image);
            let clear_color_image_info = ClearColorImageInfo {
                clear_value: self.clear_value,
                ..ClearColorImageInfo::image(image)
            };

            builder.clear_color_image(clear_color_image_info).unwrap();*/

            builder
                .clear_attachments(
                    [ClearAttachment::Color {
                        color_attachment: 0,
                        clear_value: self.clear_value,
                    }],
                    [ClearRect {
                        offset: [0; 2],
                        extent: dimensions,
                        array_layers: 0..1,
                    }],
                )
                .unwrap();
        }

        self.state = RenderTargetState::Rendering(builder);
    }

    #[profile("finish buffer")]
    pub fn finish(&mut self, font_future: Option<FenceSignalFuture<Box<dyn GpuFuture>>>) {
        if let Some(mut future) = font_future {
            #[cfg(feature = "debug")]
            profile_block!("wait for font future");

            future.wait(None).unwrap();
            future.cleanup_finished();
        }

        let mut builder = self.state.take_builder();
        builder.end_render_pass().unwrap();

        let command_buffer = builder.build().unwrap();
        let semaphore = command_buffer
            .execute(self.queue.clone())
            .unwrap()
            .boxed()
            .then_signal_semaphore_and_flush()
            .unwrap();

        self.state = RenderTargetState::Semaphore(semaphore);
        self.bound_subrenderer = None;
    }
}
