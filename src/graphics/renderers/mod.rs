mod deferred;
mod interface;
mod picker;
mod settings;
mod shadow;

use std::sync::Arc;

use cgmath::{Matrix4, Vector2, Vector3};
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, ClearAttachment, ClearColorImageInfo, ClearRect, CommandBufferUsage, CopyImageToBufferInfo,
    PrimaryAutoCommandBuffer, PrimaryCommandBufferAbstract, RenderPassBeginInfo, SubpassContents,
};
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::{Device, Queue};
use vulkano::format::{ClearColorValue, ClearValue, Format};
use vulkano::image::view::{ImageView, ImageViewCreateInfo};
use vulkano::image::{AttachmentImage, ImageAccess, ImageUsage, ImageViewAbstract, SampleCount, SwapchainImage};
use vulkano::pipeline::graphics::color_blend::{AttachmentBlend, BlendFactor, BlendOp};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass};
use vulkano::swapchain::{
    acquire_next_image, AcquireError, ColorSpace, PresentInfo, PresentMode, Surface, SurfaceInfo, Swapchain, SwapchainCreateInfo,
    SwapchainPresentInfo,
};
use vulkano::sync::{FenceSignalFuture, GpuFuture, SemaphoreSignalFuture};
use winit::window::Window;

pub use self::deferred::DeferredRenderer;
use self::deferred::DeferredSubrenderer;
pub use self::interface::InterfaceRenderer;
pub use self::picker::PickerRenderer;
use self::picker::PickerSubrenderer;
pub use self::settings::RenderSettings;
pub use self::shadow::ShadowRenderer;
use super::MemoryAllocator;
use crate::graphics::{Camera, ImageBuffer, ModelVertexBuffer, Texture};
use crate::world::MarkerIdentifier;

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
    alpha_op: BlendOp::Add,
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
        entity_id: usize,
    ) where
        Self: Renderer;
}

#[derive(Debug)]
pub enum PickerTarget {
    Tile(u16, u16),
    Entity(u32),
    #[cfg(feature = "debug")]
    Marker(MarkerIdentifier),
}

impl From<u32> for PickerTarget {
    fn from(data: u32) -> Self {
        if data >> 31 == 1 {
            let x = ((data >> 16) as u16) ^ (1 << 15);
            let y = data as u16;
            return Self::Tile(x, y);
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

        Self::Entity(entity_id)
    }
}

impl From<PickerTarget> for u32 {
    fn from(picker_target: PickerTarget) -> Self {
        match picker_target {
            PickerTarget::Tile(x, y) => {
                let mut encoded = ((x as u32) << 16) | y as u32;
                encoded |= 1 << 31;
                encoded
            }
            PickerTarget::Entity(entity_id) => match entity_id >> 24 == 0 {
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
                Format::R8G8B8A8_SRGB,
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

    pub fn finish(&mut self, swapchain: Arc<Swapchain>, semaphore: Box<dyn GpuFuture>, image_number: usize) {
        let mut builder = self.state.take_builder();

        builder.end_render_pass().unwrap();

        let command_buffer = builder.build().unwrap();

        // TODO: make this type ImageNumber instead
        let present_info = SwapchainPresentInfo::swapchain_image_index(swapchain, image_number as u32);

        self.state = semaphore
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(self.queue.clone(), present_info)
            .boxed()
            .then_signal_fence_and_flush()
            .map(RenderTargetState::Fence)
            .unwrap_or(RenderTargetState::OutOfDate);

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

    pub fn bind_subrenderer(&mut self, subrenderer: PickerSubrenderer) -> bool {
        let already_bound = self.bound_subrenderer.contains(&subrenderer);
        self.bound_subrenderer = Some(subrenderer);
        !already_bound
    }

    pub fn unbind_subrenderer(&mut self) {
        self.bound_subrenderer = None;
    }

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

pub struct SingleRenderTarget<const F: Format, S: PartialEq, C> {
    memory_allocator: Arc<MemoryAllocator>,
    queue: Arc<Queue>,
    framebuffer: Arc<Framebuffer>,
    pub image: ImageBuffer,
    pub state: RenderTargetState,
    clear_value: C,
    bound_subrenderer: Option<S>,
}

impl<const F: Format, S: PartialEq, C> SingleRenderTarget<F, S, C> {
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
            AttachmentImage::multisampled_with_usage(&*memory_allocator, dimensions, sample_count, F, image_usage).unwrap(),
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
        }
    }

    pub fn bind_subrenderer(&mut self, subrenderer: S) -> bool {
        let already_bound = self.bound_subrenderer.contains(&subrenderer);
        self.bound_subrenderer = Some(subrenderer);
        !already_bound
    }
}

impl<const F: Format, S: PartialEq> SingleRenderTarget<F, S, ClearValue> {
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

impl<const F: Format, S: PartialEq> SingleRenderTarget<F, S, ClearColorValue> {
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

    pub fn finish(&mut self, font_future: Option<FenceSignalFuture<Box<dyn GpuFuture>>>) {
        if let Some(mut future) = font_future {
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

pub struct SwapchainHolder {
    swapchain: Arc<Swapchain>,
    swapchain_images: Vec<Arc<SwapchainImage>>,
    present_mode: PresentMode,
    window_size: [u32; 2],
    image_number: usize,
    recreate: bool,
    acquire_future: Option<Box<dyn GpuFuture>>,
}

impl SwapchainHolder {
    pub fn new(physical_device: &PhysicalDevice, device: Arc<Device>, queue: Arc<Queue>, surface: Arc<Surface>) -> Self {
        let window_size: [u32; 2] = surface.object().unwrap().downcast_ref::<Window>().unwrap().inner_size().into();
        let capabilities = physical_device
            .surface_capabilities(&surface, SurfaceInfo::default())
            .expect("failed to get surface capabilities");
        let composite_alpha = capabilities.supported_composite_alpha.iter().next().unwrap();
        let image_format = physical_device.surface_formats(&surface, SurfaceInfo::default()).unwrap()[0].0;
        let present_mode = PresentMode::Fifo;
        let image_number = 0;
        let recreate = false;
        let acquire_future = None;

        let swapchain_create_info = SwapchainCreateInfo {
            min_image_count: capabilities.min_image_count,
            image_format: Some(image_format),
            image_extent: window_size,
            image_usage: ImageUsage {
                color_attachment: true,
                ..Default::default()
            },
            composite_alpha,
            image_color_space: ColorSpace::SrgbNonLinear, // Is this really needed?
            present_mode,
            ..Default::default()
        };

        let (swapchain, swapchain_images) = Swapchain::new(device, surface, swapchain_create_info).expect("failed to create swapchain");

        Self {
            swapchain,
            swapchain_images,
            present_mode,
            window_size,
            image_number,
            recreate,
            acquire_future,
        }
    }

    pub fn acquire_next_image(&mut self) -> Result<(), ()> {
        let (image_number, suboptimal, acquire_future) = match acquire_next_image(self.swapchain.clone(), None) {
            Ok(r) => r,
            Err(AcquireError::OutOfDate) => {
                self.recreate = true;
                return Err(());
            }
            Err(e) => panic!("Failed to acquire next image: {:?}", e),
        };

        self.image_number = image_number as usize;
        self.recreate |= suboptimal;
        self.acquire_future = acquire_future.boxed().into();
        Ok(())
    }

    pub fn take_acquire_future(&mut self) -> Box<dyn GpuFuture> {
        self.acquire_future.take().unwrap()
    }

    pub fn invalidate_swapchain(&mut self) {
        self.recreate = true;
    }

    pub fn is_swapchain_invalid(&self) -> bool {
        self.recreate
    }

    pub fn recreate_swapchain(&mut self) -> Viewport {
        let swapchain_create_info = SwapchainCreateInfo {
            image_extent: self.window_size,
            present_mode: self.present_mode,
            ..self.swapchain.create_info()
        };

        let swapchain_result = self.swapchain.recreate(swapchain_create_info);

        let (swapchain, swapchain_images) = match swapchain_result {
            Ok(r) => r,
            //Err(SwapchainCreationError::UnsupportedDimensions) => return,
            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
        };

        self.swapchain = swapchain;
        self.swapchain_images = swapchain_images;
        self.recreate = false;
        self.viewport()
    }

    pub fn get_swapchain(&self) -> Arc<Swapchain> {
        self.swapchain.clone()
    }

    pub fn get_swapchain_images(&self) -> Vec<Arc<SwapchainImage>> {
        self.swapchain_images.clone()
    }

    pub fn swapchain_format(&self) -> Format {
        self.swapchain.image_format()
    }

    pub fn get_image_number(&self) -> usize {
        self.image_number
    }

    pub fn viewport(&self) -> Viewport {
        Viewport {
            origin: [0.0, 0.0],
            dimensions: self.window_size.map(|component| component as f32),
            depth_range: 0.0..1.0,
        }
    }

    pub fn set_frame_limit(&mut self, limited: bool) {
        self.present_mode = match limited {
            true => PresentMode::Fifo,
            false => PresentMode::Mailbox,
        };
        self.invalidate_swapchain();
    }

    pub fn update_window_size(&mut self, window_size: [u32; 2]) {
        self.window_size = window_size;
        self.invalidate_swapchain();
    }

    pub fn window_size(&self) -> Vector2<usize> {
        self.window_size.map(|component| component as usize).into()
    }

    pub fn window_size_u32(&self) -> [u32; 2] {
        self.window_size
    }

    pub fn window_size_f32(&self) -> Vector2<f32> {
        self.window_size.map(|component| component as f32).into()
    }
}
