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

pub struct Surface {
    device: Arc<Device>,
    surface: wgpu::Surface<'static>,
    config: SurfaceConfiguration,
    present_mode_info: PresentModeInfo,
    invalid: bool,
}

impl Surface {
    pub fn new(
        adapter: &Adapter,
        device: Arc<Device>,
        surface: wgpu::Surface<'static>,
        window_width: u32,
        window_height: u32,
        triple_buffering: bool,
        vsync: bool,
    ) -> Self {
        let window_width = window_width.max(1);
        let window_height = window_height.max(1);

        let mut config = surface.get_default_config(adapter, window_width, window_height).unwrap();

        let surfaces_formats: Vec<TextureFormat> = surface.get_capabilities(adapter).formats;

        #[cfg(feature = "debug")]
        {
            print_debug!("Supported surface formats:");
            for format in &surfaces_formats {
                print_debug!("{:?}", format);
            }
        }

        let present_mode_info = PresentModeInfo::from_adapter(adapter, &surface);

        config.format = surfaces_formats.first().copied().expect("not surface formats found");
        config.desired_maximum_frame_latency = match triple_buffering {
            true => 2,
            false => 1,
        };
        config.present_mode = match vsync {
            false if present_mode_info.supports_mailbox => PresentMode::Mailbox,
            false if present_mode_info.supports_immediate => PresentMode::Immediate,
            _ => PresentMode::Fifo,
        };

        #[cfg(feature = "debug")]
        {
            print_debug!("Surface present mode is {:?}", config.present_mode.magenta());
            print_debug!("Surface format is {:?}", config.format);
        }

        surface.configure(&device, &config);

        Self {
            device,
            surface,
            config,
            present_mode_info,
            invalid: false,
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn acquire(&mut self) -> SurfaceTexture {
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

        frame
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
        self.surface.configure(&self.device, &self.config);
    }

    pub fn set_vsync(&mut self, enabled: bool) {
        self.config.present_mode = match enabled {
            false if self.present_mode_info.supports_mailbox => PresentMode::Mailbox,
            false if self.present_mode_info.supports_immediate => PresentMode::Immediate,
            _ => PresentMode::Fifo,
        };

        #[cfg(feature = "debug")]
        print_debug!("set surface present mode to {:?}", self.config.present_mode.magenta());

        self.invalidate();
    }

    pub fn set_triple_buffering(&mut self, enabled: bool) {
        self.config.desired_maximum_frame_latency = match enabled {
            true => 2,
            false => 1,
        };
    }

    pub fn update_window_size(&mut self, window_size: ScreenSize) {
        self.config.width = window_size.width as u32;
        self.config.height = window_size.height as u32;
        self.invalidate();
    }

    pub fn present_mode_info(&self) -> PresentModeInfo {
        self.present_mode_info
    }

    pub fn format(&self) -> TextureFormat {
        self.config.format
    }

    pub fn window_size(&self) -> Vector2<usize> {
        Vector2 {
            x: self.config.width as usize,
            y: self.config.height as usize,
        }
    }

    pub fn window_screen_size(&self) -> ScreenSize {
        ScreenSize {
            width: self.config.width as f32,
            height: self.config.height as f32,
        }
    }
}
