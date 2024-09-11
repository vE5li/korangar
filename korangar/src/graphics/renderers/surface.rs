use std::sync::Arc;

use cgmath::Vector2;
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize};
use wgpu::{Adapter, Device, PresentMode, SurfaceConfiguration, SurfaceTexture, TextureFormat};

use crate::interface::layout::ScreenSize;

#[derive(Debug, Clone, Copy)]
pub struct PresentModeInfo {
    /// Vsync On (Fast)
    pub supports_mailbox: bool,
    /// Vsync Off
    pub supports_immediate: bool,
}

impl PresentModeInfo {
    pub fn from_adapter(adapter: &Adapter, surface: &wgpu::Surface) -> PresentModeInfo {
        let mut present_mode_info = PresentModeInfo {
            supports_immediate: false,
            supports_mailbox: false,
        };

        surface
            .get_capabilities(adapter)
            .present_modes
            .iter()
            .for_each(|present_mode| match present_mode {
                PresentMode::Mailbox => present_mode_info.supports_mailbox = true,
                PresentMode::Immediate => present_mode_info.supports_immediate = true,
                _ => {}
            });

        present_mode_info
    }
}

pub struct Surface<'window> {
    device: Arc<Device>,
    surface: wgpu::Surface<'window>,
    config: SurfaceConfiguration,
    present_mode_info: PresentModeInfo,
    frame_number: usize,
    max_frame_count: usize,
    window_width: u32,
    window_height: u32,
    invalid: bool,
}

impl<'window> Surface<'window> {
    pub fn new(adapter: Adapter, device: Arc<Device>, surface: wgpu::Surface<'window>, window_width: u32, window_height: u32) -> Self {
        let window_width = window_width.max(1);
        let window_height = window_height.max(1);

        let mut config = surface.get_default_config(&adapter, window_width, window_height).unwrap();
        let recreate = false;

        let surfaces_formats: Vec<TextureFormat> = surface.get_capabilities(&adapter).formats;

        #[cfg(feature = "debug")]
        {
            print_debug!("Supported surface formats:");
            for format in &surfaces_formats {
                print_debug!("{:?}", format);
            }
        }

        let srgb_formats: Vec<TextureFormat> = surfaces_formats.iter().copied().filter(|format| format.is_srgb()).collect();
        let srgb_format = *srgb_formats.first().expect("Surface does not support sRGB");

        config.format = srgb_format;
        config.view_formats.push(srgb_format);

        // Fifo is supported on all platforms.
        let present_mode_info = PresentModeInfo::from_adapter(&adapter, &surface);
        config.present_mode = PresentMode::Fifo;

        #[cfg(feature = "debug")]
        print_debug!("Surface format is {:?}", config.format);

        surface.configure(&device, &config);

        let max_frame_count = config.desired_maximum_frame_latency as usize;

        Self {
            device,
            surface,
            config,
            present_mode_info,
            frame_number: 0,
            max_frame_count,
            window_width,
            window_height,
            invalid: recreate,
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn acquire(&mut self) -> (usize, SurfaceTexture) {
        self.frame_number = (self.frame_number + 1) % (self.max_frame_count - 1);

        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            // On timeout, we will just try again.
            Err(wgpu::SurfaceError::Timeout) => self.surface
                .get_current_texture()
                .expect("Failed to acquire next surface texture!"),
            Err(
                // If the surface is outdated, or was lost, reconfigure it.
                wgpu::SurfaceError::Outdated
                | wgpu::SurfaceError::Lost
                // If OutOfMemory happens, reconfiguring may not help, but we might as well try.
                | wgpu::SurfaceError::OutOfMemory,
            ) => {
                self.surface.configure(&self.device, &self.config);
                self.surface
                    .get_current_texture()
                    .expect("Failed to acquire next surface texture!")
            }
        };

        if frame.suboptimal {
            self.invalid = true;
        }

        (self.frame_number, frame)
    }

    pub fn invalidate(&mut self) {
        self.invalid = true;
    }

    pub fn is_invalid(&self) -> bool {
        self.invalid
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn reconfigure(&mut self) {
        self.invalid = false;
        self.config.width = self.window_width.max(1);
        self.config.height = self.window_height.max(1);
        self.surface.configure(&self.device, &self.config);
    }

    pub fn set_frame_limit(&mut self, limited: bool) {
        self.config.present_mode = match limited {
            false if self.present_mode_info.supports_mailbox => PresentMode::Mailbox,
            false if self.present_mode_info.supports_immediate => PresentMode::Immediate,
            _ => PresentMode::Fifo,
        };

        #[cfg(feature = "debug")]
        print_debug!("set surface present mode to {:?}", self.config.present_mode.magenta());

        self.invalidate();
    }

    pub fn update_window_size(&mut self, window_size: [u32; 2]) {
        self.window_width = window_size[0];
        self.window_height = window_size[1];
        self.invalidate();
    }

    pub fn window_size(&self) -> Vector2<usize> {
        Vector2 {
            x: self.window_width as usize,
            y: self.window_height as usize,
        }
    }

    pub fn max_frame_count(&self) -> usize {
        self.max_frame_count
    }

    pub fn frame_number(&self) -> usize {
        self.frame_number
    }

    pub fn present_mode_info(&self) -> PresentModeInfo {
        self.present_mode_info
    }

    pub fn format(&self) -> TextureFormat {
        self.config.format
    }

    pub fn window_size_u32(&self) -> [u32; 2] {
        [self.window_width, self.window_height]
    }

    pub fn window_screen_size(&self) -> ScreenSize {
        ScreenSize {
            width: self.window_width as f32,
            height: self.window_height as f32,
        }
    }
}
