use std::sync::Arc;

use cgmath::Vector2;
use procedural::profile;
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::image::{ImageUsage, SwapchainImage};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::swapchain::{acquire_next_image, AcquireError, ColorSpace, PresentMode, Surface, SurfaceInfo, Swapchain, SwapchainCreateInfo};
use vulkano::sync::GpuFuture;
use winit::window::Window;

#[cfg(feature = "debug")]
use crate::debug::*;

#[derive(Debug, Clone, Copy)]
pub struct PresentModeInfo {
    pub supports_immediate: bool,
    pub supports_mailbox: bool,
}

impl PresentModeInfo {
    pub fn from_device(physical_device: &PhysicalDevice, surface: &Surface) -> PresentModeInfo {
        let mut presend_mode_info = PresentModeInfo {
            supports_immediate: false,
            supports_mailbox: false,
        };

        physical_device
            .surface_present_modes(surface)
            .expect("failed to get surface present modes")
            .for_each(|presend_mode| match presend_mode {
                PresentMode::Immediate => presend_mode_info.supports_immediate = true,
                PresentMode::Mailbox => presend_mode_info.supports_mailbox = true,
                _ => {}
            });

        presend_mode_info
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
    pub fn new(physical_device: &PhysicalDevice, device: Arc<Device>, _queue: Arc<Queue>, surface: Arc<Surface>) -> Self {
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

    #[profile]
    pub fn acquire_next_image(&mut self) -> Result<(), ()> {
        let (image_number, suboptimal, acquire_future) = match acquire_next_image(self.swapchain.clone(), None) {
            Ok(image) => image,
            Err(AcquireError::OutOfDate) => {
                self.recreate = true;
                return Err(());
            }
            Err(error) => panic!("Failed to acquire next image: {error:?}"),
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

    #[profile]
    pub fn recreate_swapchain(&mut self) -> Viewport {
        let swapchain_create_info = SwapchainCreateInfo {
            image_extent: self.window_size,
            present_mode: self.present_mode,
            ..self.swapchain.create_info()
        };

        let swapchain_result = self.swapchain.recreate(swapchain_create_info);

        let (swapchain, swapchain_images) = match swapchain_result {
            Ok(swapchain) => swapchain,
            //Err(SwapchainCreationError::UnsupportedDimensions) => return,
            Err(error) => panic!("Failed to recreate swapchain: {error:?}"),
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

    pub fn set_frame_limit(&mut self, presend_mode_info: PresentModeInfo, limited: bool) {
        self.present_mode = match limited {
            false if presend_mode_info.supports_mailbox => PresentMode::Mailbox,
            false if presend_mode_info.supports_immediate => PresentMode::Immediate,
            _ => PresentMode::Fifo,
        };

        #[cfg(feature = "debug")]
        Timer::new_dynamic(format!(
            "set swapchain present mode to {}{:?}{}",
            MAGENTA, self.present_mode, NONE
        ))
        .stop();

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
