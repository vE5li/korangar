mod deferred;
mod interface;
mod picker;
mod settings;
mod shadow;

use std::sync::Arc;

use cgmath::{Matrix4, Vector2, Vector3};
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, PrimaryCommandBuffer, SubpassContents,
};
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::{Device, Queue};
use vulkano::format::{ClearValue, Format};
use vulkano::image::view::ImageView;
use vulkano::image::{AttachmentImage, ImageUsage, SampleCount, SwapchainImage};
use vulkano::pipeline::graphics::color_blend::{AttachmentBlend, BlendFactor, BlendOp};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{Framebuffer, RenderPass};
use vulkano::swapchain::{acquire_next_image, AcquireError, ColorSpace, PresentMode, Surface, Swapchain};
use vulkano::sync::{FenceSignalFuture, GpuFuture, SemaphoreSignalFuture};
use winit::window::Window;

pub use self::deferred::DeferredRenderer;
use self::deferred::DeferredSubrenderer;
pub use self::interface::InterfaceRenderer;
pub use self::picker::PickerRenderer;
use self::picker::PickerSubrenderer;
pub use self::settings::RenderSettings;
pub use self::shadow::ShadowRenderer;
use crate::graphics::{Camera, ImageBuffer, ModelVertexBuffer, Texture};

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
        textures: &Vec<Texture>,
        world_matrix: Matrix4<f32>,
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
        entity_id: usize,
    ) where
        Self: Renderer;
}

#[derive(Debug)]
pub enum PickerTarget {
    Tile(u16, u16),
    Entity(u32),
    /*ObjectMarker(u16),
    LightSoorceMarker(u16),
    SoundSourceMarker(u16),
    EffectSourceMarker(u16),
    ParticleMarker(u16, u8),*/
}

impl From<u32> for PickerTarget {

    fn from(data: u32) -> Self {

        if data >> 31 == 1 {

            let x = ((data >> 16) as u16) ^ (1 << 15);
            let y = data as u16;
            return Self::Tile(x, y);
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
        }
    }
}

pub trait MarkerRenderer {

    fn render_marker(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, position: Vector3<f32>, hovered: bool)
    where
        Self: Renderer;
}

pub enum RenderTargetState {
    Ready,
    Rendering(AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>),
    Semaphore(SemaphoreSignalFuture<Box<dyn GpuFuture>>),
    Fence(FenceSignalFuture<Box<dyn GpuFuture>>),
}

unsafe impl Send for RenderTargetState {}

impl RenderTargetState {

    pub fn get_builder(&mut self) -> &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer> {

        let RenderTargetState::Rendering(builder) = self else {
            panic!("render target is not in the render state");
        };

        builder
    }

    pub fn take_builder(&mut self) -> AutoCommandBufferBuilder<PrimaryAutoCommandBuffer> {

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
    device: Arc<Device>,
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
        device: Arc<Device>,
        queue: Arc<Queue>,
        render_pass: Arc<RenderPass>,
        swapchain_image: Arc<SwapchainImage<Window>>,
        dimensions: [u32; 2],
    ) -> Self {

        let color_image_usage = ImageUsage {
            sampled: true,
            color_attachment: true,
            input_attachment: true,
            ..ImageUsage::none()
        };

        let depth_image_usage = ImageUsage {
            sampled: true,
            depth_stencil_attachment: true,
            input_attachment: true,
            ..ImageUsage::none()
        };

        let diffuse_image = ImageView::new(Arc::new(
            AttachmentImage::multisampled_with_usage(
                device.clone(),
                dimensions,
                SampleCount::Sample4,
                Format::R32G32B32A32_SFLOAT,
                color_image_usage,
            )
            .unwrap(),
        ))
        .unwrap();
        let normal_image = ImageView::new(Arc::new(
            AttachmentImage::multisampled_with_usage(
                device.clone(),
                dimensions,
                SampleCount::Sample4,
                Format::R16G16B16A16_SFLOAT,
                color_image_usage,
            )
            .unwrap(),
        ))
        .unwrap();
        let water_image = ImageView::new(Arc::new(
            AttachmentImage::multisampled_with_usage(
                device.clone(),
                dimensions,
                SampleCount::Sample4,
                Format::R8G8B8A8_SRGB,
                color_image_usage,
            )
            .unwrap(),
        ))
        .unwrap();
        let depth_image = ImageView::new(Arc::new(
            AttachmentImage::multisampled_with_usage(
                device.clone(),
                dimensions,
                SampleCount::Sample4,
                Format::D32_SFLOAT,
                depth_image_usage,
            )
            .unwrap(),
        ))
        .unwrap();

        let framebuffer = Framebuffer::start(render_pass)
            .add(ImageView::new(swapchain_image).unwrap())
            .unwrap()
            .add(diffuse_image.clone())
            .unwrap()
            .add(normal_image.clone())
            .unwrap()
            .add(water_image.clone())
            .unwrap()
            .add(depth_image.clone())
            .unwrap()
            .build()
            .unwrap();

        let state = RenderTargetState::Ready;
        let bound_subrenderer = None;

        Self {
            device,
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

        let mut builder =
            AutoCommandBufferBuilder::primary(self.device.clone(), self.queue.family(), CommandBufferUsage::OneTimeSubmit).unwrap();

        builder
            .begin_render_pass(
                self.framebuffer.clone(),
                SubpassContents::Inline,
                [
                    ClearValue::Float([0.0, 0.0, 0.0, 1.0]),
                    ClearValue::Float([0.0, 0.0, 0.0, 1.0]),
                    ClearValue::Float([0.0, 0.0, 0.0, 1.0]),
                    ClearValue::Float([0.0, 0.0, 0.0, 1.0]),
                    ClearValue::Depth(1.0),
                ],
            )
            .unwrap();

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

    pub fn finish(&mut self, swapchain: Arc<Swapchain<Window>>, semaphore: Box<dyn GpuFuture>, image_number: usize) {

        let mut builder = self.state.take_builder();

        builder.end_render_pass().unwrap();

        let command_buffer = builder.build().unwrap();

        let fence = semaphore
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(self.queue.clone(), swapchain, image_number)
            .boxed()
            .then_signal_fence_and_flush()
            .unwrap();

        self.state = RenderTargetState::Fence(fence);
        self.bound_subrenderer = None;
    }
}

pub struct PickerRenderTarget {
    device: Arc<Device>,
    queue: Arc<Queue>,
    framebuffer: Arc<Framebuffer>,
    pub image: ImageBuffer,
    pub buffer: Arc<CpuAccessibleBuffer<[u32]>>,
    pub state: RenderTargetState,
    bound_subrenderer: Option<PickerSubrenderer>,
}

impl PickerRenderTarget {

    pub fn new(device: Arc<Device>, queue: Arc<Queue>, render_pass: Arc<RenderPass>, dimensions: [u32; 2]) -> Self {

        let image_usage = ImageUsage {
            sampled: true,
            transfer_source: true,
            color_attachment: true,
            ..ImageUsage::none()
        };

        let depth_image_usage = ImageUsage {
            depth_stencil_attachment: true,
            ..ImageUsage::none()
        };

        let image = ImageView::new(Arc::new(
            AttachmentImage::with_usage(device.clone(), dimensions, Format::R32_UINT, image_usage).unwrap(),
        ))
        .unwrap();
        let depth_buffer = ImageView::new(Arc::new(
            AttachmentImage::with_usage(device.clone(), dimensions, Format::D16_UNORM, depth_image_usage).unwrap(),
        ))
        .unwrap();
        let framebuffer = Framebuffer::start(render_pass)
            .add(image.clone())
            .unwrap()
            .add(depth_buffer)
            .unwrap()
            .build()
            .unwrap();

        let buffer = unsafe {
            CpuAccessibleBuffer::uninitialized_array(
                device.clone(),
                dimensions[0] as u64 * dimensions[1] as u64,
                BufferUsage::transfer_destination(),
                false,
            )
            .unwrap()
        };
        let state = RenderTargetState::Ready;
        let bound_subrenderer = None;

        Self {
            device,
            queue,
            framebuffer,
            image,
            buffer,
            state,
            bound_subrenderer,
        }
    }

    pub fn start(&mut self) {

        let mut builder =
            AutoCommandBufferBuilder::primary(self.device.clone(), self.queue.family(), CommandBufferUsage::OneTimeSubmit).unwrap();

        builder
            .begin_render_pass(
                self.framebuffer.clone(),
                SubpassContents::Inline,
                [ClearValue::Uint([0; 4]), ClearValue::Depth(1.0)],
            )
            .unwrap();

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
            .copy_image_to_buffer(self.image.image().clone(), self.buffer.clone())
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

pub struct SingleRenderTarget<const F: Format, S: PartialEq> {
    device: Arc<Device>,
    queue: Arc<Queue>,
    framebuffer: Arc<Framebuffer>,
    pub image: ImageBuffer,
    pub state: RenderTargetState,
    clear_value: ClearValue,
    bound_subrenderer: Option<S>,
}

impl<const F: Format, S: PartialEq> SingleRenderTarget<F, S> {

    pub fn new(
        device: Arc<Device>,
        queue: Arc<Queue>,
        render_pass: Arc<RenderPass>,
        dimensions: [u32; 2],
        sample_count: SampleCount,
        image_usage: ImageUsage,
        clear_value: ClearValue,
    ) -> Self {

        let image = ImageView::new(Arc::new(
            AttachmentImage::multisampled_with_usage(device.clone(), dimensions, sample_count, F, image_usage).unwrap(),
        ))
        .unwrap();
        let framebuffer = Framebuffer::start(render_pass).add(image.clone()).unwrap().build().unwrap();

        let state = RenderTargetState::Ready;
        let bound_subrenderer = None;

        Self {
            device,
            queue,
            framebuffer,
            image,
            state,
            clear_value,
            bound_subrenderer,
        }
    }

    pub fn start(&mut self) {

        let mut builder =
            AutoCommandBufferBuilder::primary(self.device.clone(), self.queue.family(), CommandBufferUsage::OneTimeSubmit).unwrap();

        builder
            .begin_render_pass(self.framebuffer.clone(), SubpassContents::Inline, [self.clear_value])
            .unwrap();

        self.state = RenderTargetState::Rendering(builder);
    }

    pub fn start_interface(&mut self, clear_interface: bool) {

        // TODO:

        let mut builder =
            AutoCommandBufferBuilder::primary(self.device.clone(), self.queue.family(), CommandBufferUsage::OneTimeSubmit).unwrap();

        if clear_interface {
            builder.clear_color_image(self.image.image().clone(), self.clear_value).unwrap();
        }

        builder
            .begin_render_pass(self.framebuffer.clone(), SubpassContents::Inline, [ClearValue::None])
            .unwrap();

        self.state = RenderTargetState::Rendering(builder);
    }

    pub fn bind_subrenderer(&mut self, subrenderer: S) -> bool {

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
    swapchain: Arc<Swapchain<Window>>,
    swapchain_images: Vec<Arc<SwapchainImage<Window>>>,
    present_mode: PresentMode,
    window_size: [u32; 2],
    image_number: usize,
    recreate: bool,
    acquire_future: Option<Box<dyn GpuFuture>>,
}

impl SwapchainHolder {

    pub fn new(physical_device: &PhysicalDevice, device: Arc<Device>, queue: Arc<Queue>, surface: Arc<Surface<Window>>) -> Self {

        let window_size: [u32; 2] = surface.window().inner_size().into();
        let capabilities = surface.capabilities(*physical_device).expect("failed to get surface capabilities");
        let composite_alpha = capabilities.supported_composite_alpha.iter().next().unwrap();
        let format = capabilities.supported_formats[0].0;
        let present_mode = PresentMode::Fifo;
        let image_number = 0;
        let recreate = false;
        let acquire_future = None;

        let (swapchain, swapchain_images) = Swapchain::start(device, surface)
            .num_images(capabilities.min_image_count)
            .format(format)
            .dimensions(window_size)
            .usage(ImageUsage::color_attachment())
            .sharing_mode(&queue)
            .composite_alpha(composite_alpha)
            .color_space(ColorSpace::SrgbNonLinear)
            .present_mode(present_mode)
            .build()
            .expect("failed to create swapchain");

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

        self.image_number = image_number;
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

        let swapchain_result = self
            .swapchain
            .recreate()
            .dimensions(self.window_size)
            .present_mode(self.present_mode)
            .build();

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

    pub fn get_swapchain(&self) -> Arc<Swapchain<Window>> {
        self.swapchain.clone()
    }

    pub fn get_swapchain_images(&self) -> Vec<Arc<SwapchainImage<Window>>> {
        self.swapchain_images.clone()
    }

    pub fn swapchain_format(&self) -> Format {
        self.swapchain.format()
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
