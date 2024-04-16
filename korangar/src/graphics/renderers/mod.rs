macro_rules! vertex_shader {
    ($path:literal) => {
        mod vertex_shader {
            vulkano_shaders::shader! {
                ty: "vertex",
                path: $path
            }

            pub fn entry_point(device: &std::sync::Arc<vulkano::device::Device>) -> vulkano::shader::EntryPoint {
                load(device.clone()).unwrap().entry_point("main").unwrap()
            }
        }
    };
}

macro_rules! fragment_shader {
    ($path:literal) => {
        mod fragment_shader {
            vulkano_shaders::shader! {
                ty: "fragment",
                path: $path
            }

            pub fn entry_point(device: &std::sync::Arc<vulkano::device::Device>) -> vulkano::shader::EntryPoint {
                load(device.clone()).unwrap().entry_point("main").unwrap()
            }
        }
    };
}

mod deferred;
mod image;
mod interface;
mod picker;
mod pipeline;
mod sampler;
#[cfg(feature = "debug")]
mod settings;
mod shadow;
mod swapchain;

use std::marker::{ConstParamTy, PhantomData};
use std::sync::Arc;

use cgmath::{Matrix4, Vector2, Vector3};
use korangar_procedural::profile;
use option_ext::OptionExt;
use ragnarok_networking::EntityId;
use vulkano::buffer::{Buffer, BufferUsage, Subbuffer};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, ClearAttachment, ClearRect, CommandBufferUsage, CopyImageToBufferInfo, PrimaryAutoCommandBuffer,
    PrimaryCommandBufferAbstract, RenderPassBeginInfo, SubpassBeginInfo, SubpassEndInfo,
};
use vulkano::device::Queue;
use vulkano::format::{ClearColorValue, ClearValue, Format};
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageUsage, SampleCount};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::pipeline::graphics::color_blend::{AttachmentBlend, BlendFactor, BlendOp};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass};
use vulkano::swapchain::{Swapchain, SwapchainPresentInfo};
use vulkano::sync::future::{FenceSignalFuture, SemaphoreSignalFuture};
use vulkano::sync::GpuFuture;
use vulkano::Validated;

pub use self::deferred::DeferredRenderer;
use self::deferred::DeferredSubrenderer;
use self::image::{AttachmentImageFactory, AttachmentImageType};
pub use self::interface::InterfaceRenderer;
use self::picker::PickerSubrenderer;
pub use self::picker::{PickerRenderer, PickerTarget};
#[cfg(feature = "debug")]
pub use self::settings::RenderSettings;
pub use self::shadow::{ShadowDetail, ShadowRenderer};
pub use self::swapchain::{PresentModeInfo, SwapchainHolder};
use super::{Color, MemoryAllocator, ModelVertex};
#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::Camera;
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

pub const LIGHT_ATTACHMENT_BLEND: AttachmentBlend = AttachmentBlend {
    color_blend_op: BlendOp::Add,
    src_color_blend_factor: BlendFactor::One,
    dst_color_blend_factor: BlendFactor::One,
    alpha_blend_op: BlendOp::Max,
    src_alpha_blend_factor: BlendFactor::One,
    dst_alpha_blend_factor: BlendFactor::One,
};

pub const WATER_ATTACHMENT_BLEND: AttachmentBlend = AttachmentBlend {
    color_blend_op: BlendOp::ReverseSubtract,
    src_color_blend_factor: BlendFactor::One,
    dst_color_blend_factor: BlendFactor::One,
    alpha_blend_op: BlendOp::Max,
    src_alpha_blend_factor: BlendFactor::One,
    dst_alpha_blend_factor: BlendFactor::One,
};

pub const INTERFACE_ATTACHMENT_BLEND: AttachmentBlend = AttachmentBlend {
    color_blend_op: BlendOp::Add,
    src_color_blend_factor: BlendFactor::SrcAlpha,
    dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
    alpha_blend_op: BlendOp::Max,
    src_alpha_blend_factor: BlendFactor::SrcAlpha,
    dst_alpha_blend_factor: BlendFactor::DstAlpha,
};

pub const EFFECT_ATTACHMENT_BLEND: AttachmentBlend = AttachmentBlend {
    color_blend_op: BlendOp::Max,
    src_color_blend_factor: BlendFactor::One,
    dst_color_blend_factor: BlendFactor::One,
    alpha_blend_op: BlendOp::Max,
    src_alpha_blend_factor: BlendFactor::One,
    dst_alpha_blend_factor: BlendFactor::One,
};

pub trait Renderer {
    type Target;
}

#[derive(Eq, PartialEq)]
struct SubpassAttachments {
    color: u32,
    depth: u32,
}

impl ConstParamTy for SubpassAttachments {}

pub trait GeometryRenderer {
    fn render_geometry(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        vertex_buffer: Subbuffer<[ModelVertex]>,
        textures: &[Arc<ImageView>],
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
        texture: Arc<ImageView>,
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

pub trait SpriteRenderer {
    fn render_sprite(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        texture: Arc<ImageView>,
        position: ScreenPosition,
        size: ScreenSize,
        screen_clip: ScreenClip,
        color: Color,
        smooth: bool,
    ) where
        Self: Renderer;
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
    Rendering(AutoCommandBufferBuilder<PrimaryAutoCommandBuffer<MemoryAllocator>, MemoryAllocator>),
    Semaphore(SemaphoreSignalFuture<Box<dyn GpuFuture>>),
    Fence(FenceSignalFuture<Box<dyn GpuFuture>>),
    OutOfDate,
}

unsafe impl Send for RenderTargetState {}

impl RenderTargetState {
    pub fn get_builder(&mut self) -> &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer<MemoryAllocator>, MemoryAllocator> {
        let RenderTargetState::Rendering(builder) = self else {
            panic!("render target is not in the render state");
        };

        builder
    }

    pub fn take_builder(&mut self) -> AutoCommandBufferBuilder<PrimaryAutoCommandBuffer<MemoryAllocator>, MemoryAllocator> {
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
    diffuse_image: Arc<ImageView>,
    normal_image: Arc<ImageView>,
    water_image: Arc<ImageView>,
    depth_image: Arc<ImageView>,
    pub state: RenderTargetState,
    bound_subrenderer: Option<DeferredSubrenderer>,
}

impl DeferredRenderTarget {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        queue: Arc<Queue>,
        render_pass: Arc<RenderPass>,
        swapchain_image: Arc<Image>,
        dimensions: [u32; 2],
    ) -> Self {
        let image_factory = AttachmentImageFactory::new(&memory_allocator, dimensions, SampleCount::Sample4);

        let diffuse_image = image_factory.new_image(Format::R32G32B32A32_SFLOAT, AttachmentImageType::InputColor);
        let normal_image = image_factory.new_image(Format::R16G16B16A16_SFLOAT, AttachmentImageType::InputColor);
        let water_image = image_factory.new_image(Format::R8G8B8A8_UNORM, AttachmentImageType::InputColor);
        let depth_image = image_factory.new_image(Format::D32_SFLOAT, AttachmentImageType::InputDepth);

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

        builder
            .begin_render_pass(render_pass_begin_info, SubpassBeginInfo::default())
            .unwrap();

        self.state = RenderTargetState::Rendering(builder);
        self.bound_subrenderer = None;
    }

    pub fn bind_subrenderer(&mut self, subrenderer: DeferredSubrenderer) -> bool {
        let already_bound = self.bound_subrenderer.contains(&subrenderer);
        self.bound_subrenderer = Some(subrenderer);
        !already_bound
    }

    pub fn lighting_pass(&mut self) {
        self.state
            .get_builder()
            .next_subpass(SubpassEndInfo::default(), SubpassBeginInfo::default())
            .unwrap();
    }

    #[profile("finish swapchain image")]
    pub fn finish(&mut self, swapchain: Arc<Swapchain>, semaphore: Box<dyn GpuFuture>, image_number: usize) {
        let mut builder = self.state.take_builder();

        #[cfg(feature = "debug")]
        let end_render_pass_measurement = start_measurement("end render pass");

        builder.end_render_pass(SubpassEndInfo::default()).unwrap();

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
            .map_err(Validated::unwrap)
            .unwrap_or(RenderTargetState::OutOfDate);

        #[cfg(feature = "debug")]
        flush_measurement.stop();
    }
}

pub struct PickerRenderTarget {
    memory_allocator: Arc<MemoryAllocator>,
    queue: Arc<Queue>,
    framebuffer: Arc<Framebuffer>,
    pub image: Arc<ImageView>,
    pub buffer: Subbuffer<[u32]>,
    pub state: RenderTargetState,
    bound_subrenderer: Option<PickerSubrenderer>,
}

impl PickerRenderTarget {
    pub fn new(memory_allocator: Arc<MemoryAllocator>, queue: Arc<Queue>, render_pass: Arc<RenderPass>, dimensions: [u32; 2]) -> Self {
        let image_factory = AttachmentImageFactory::new(&memory_allocator, dimensions, SampleCount::Sample1);

        let image = image_factory.new_image(Format::R32_UINT, AttachmentImageType::CopyColor);
        let depth_image = image_factory.new_image(Format::D16_UNORM, AttachmentImageType::Depth);

        let framebuffer_create_info = FramebufferCreateInfo {
            attachments: vec![image.clone(), depth_image],
            ..Default::default()
        };

        let framebuffer = Framebuffer::new(render_pass, framebuffer_create_info).unwrap();

        let buffer = Buffer::new_slice(
            &*memory_allocator,
            vulkano::buffer::BufferCreateInfo {
                usage: BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST | MemoryTypeFilter::HOST_RANDOM_ACCESS,
                ..Default::default()
            },
            dimensions[0] as u64 * dimensions[1] as u64,
        )
        .unwrap();

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
        let mut builder = AutoCommandBufferBuilder::<_, MemoryAllocator>::primary(
            &*self.memory_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let render_pass_begin_info = RenderPassBeginInfo {
            clear_values: vec![Some(ClearValue::Uint([0; 4])), Some(ClearValue::Depth(1.0))],
            ..RenderPassBeginInfo::framebuffer(self.framebuffer.clone())
        };

        builder
            .begin_render_pass(render_pass_begin_info, SubpassBeginInfo::default())
            .unwrap();

        self.state = RenderTargetState::Rendering(builder);
        self.bound_subrenderer = None;
    }

    #[profile]
    pub fn bind_subrenderer(&mut self, subrenderer: PickerSubrenderer) -> bool {
        let already_bound = self.bound_subrenderer.contains(&subrenderer);
        self.bound_subrenderer = Some(subrenderer);
        !already_bound
    }

    #[profile("finish buffer")]
    pub fn finish(&mut self) {
        let mut builder = self.state.take_builder();

        builder.end_render_pass(SubpassEndInfo::default()).unwrap();
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
    }
}

pub trait IntoFormat {
    fn into_format() -> Format;
}

pub struct SingleRenderTarget<F: IntoFormat, S: PartialEq, C> {
    memory_allocator: Arc<MemoryAllocator>,
    queue: Arc<Queue>,
    framebuffer: Arc<Framebuffer>,
    pub image: Arc<ImageView>,
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
        let image = Image::new(
            &*memory_allocator,
            ImageCreateInfo {
                format: F::into_format(),
                extent: [dimensions[0], dimensions[1], 1],
                samples: sample_count,
                usage: image_usage,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap();
        let image = ImageView::new_default(image).unwrap();

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

        builder
            .begin_render_pass(render_pass_begin_info, SubpassBeginInfo::default())
            .unwrap();

        self.state = RenderTargetState::Rendering(builder);
        self.bound_subrenderer = None;
    }

    #[profile("finalize buffer")]
    pub fn finish(&mut self) {
        let mut builder = self.state.take_builder();

        builder.end_render_pass(SubpassEndInfo::default()).unwrap();

        let command_buffer = builder.build().unwrap();
        let semaphore = command_buffer
            .execute(self.queue.clone())
            .unwrap()
            .boxed()
            .then_signal_semaphore_and_flush()
            .unwrap();

        self.state = RenderTargetState::Semaphore(semaphore);
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

        builder
            .begin_render_pass(render_pass_begin_info, SubpassBeginInfo::default())
            .unwrap();

        if clear_interface {
            builder
                .clear_attachments(
                    [ClearAttachment::Color {
                        color_attachment: 0,
                        clear_value: self.clear_value,
                    }]
                    .into_iter()
                    .collect(),
                    [ClearRect {
                        offset: [0; 2],
                        extent: dimensions,
                        array_layers: 0..1,
                    }]
                    .into_iter()
                    .collect(),
                )
                .unwrap();
        }

        self.state = RenderTargetState::Rendering(builder);
        self.bound_subrenderer = None;
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
        builder.end_render_pass(SubpassEndInfo::default()).unwrap();

        let command_buffer = builder.build().unwrap();
        let semaphore = command_buffer
            .execute(self.queue.clone())
            .unwrap()
            .boxed()
            .then_signal_semaphore_and_flush()
            .unwrap();

        self.state = RenderTargetState::Semaphore(semaphore);
    }
}
