use cgmath::Vector2;
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use rust_state::RustState;
use wgpu::{
    Adapter, CurrentSurfaceTexture, Device, PresentMode, SurfaceCapabilities, SurfaceConfiguration, SurfaceTexture, TextureFormat,
    TextureUsages,
};

use crate::graphics::ScreenSize;

#[derive(RustState, Debug, Clone, Copy)]
pub struct PresentModeInfo {
    /// Vsync On (Fast)
    pub supports_mailbox: bool,
    /// Vsync Off
    pub supports_immediate: bool,
}

impl PresentModeInfo {
    fn from_capabilities(capabilities: &SurfaceCapabilities) -> PresentModeInfo {
        let mut present_mode_info = PresentModeInfo {
            supports_immediate: false,
            supports_mailbox: false,
        };

        capabilities.present_modes.iter().for_each(|present_mode| match present_mode {
            PresentMode::Mailbox => present_mode_info.supports_mailbox = true,
            PresentMode::Immediate => present_mode_info.supports_immediate = true,
            _ => {}
        });

        present_mode_info
    }
}

/// Select only configurations advertised by the adapter for this surface.
/// In particular, an adapter found on a surfaceless EGL display is not evidence
/// that it can present to a native window.
fn surface_configuration(
    capabilities: &SurfaceCapabilities,
    width: u32,
    height: u32,
    triple_buffering: bool,
    vsync: bool,
) -> Result<SurfaceConfiguration, String> {
    let format = capabilities.formats.first().copied().ok_or(
        "No presentable surface formats. On GLES, the instance must use the window's native display connection; a surfaceless adapter \
         cannot present",
    )?;
    if capabilities.present_modes.is_empty() {
        return Err("No presentation modes are supported by this adapter and window surface".into());
    }
    // Fifo is wgpu's guaranteed vsync mode and is required by our runtime toggle.
    if !capabilities.present_modes.contains(&PresentMode::Fifo) {
        return Err("The surface does not advertise the required Fifo (vsync) presentation mode".into());
    }
    if !capabilities.usages.contains(TextureUsages::RENDER_ATTACHMENT) {
        return Err("The surface does not support RENDER_ATTACHMENT usage".into());
    }
    let alpha_mode = capabilities
        .alpha_modes
        .first()
        .copied()
        .ok_or("No surface alpha modes are supported")?;
    let present_mode_info = PresentModeInfo::from_capabilities(capabilities);
    let present_mode = match vsync {
        false if present_mode_info.supports_mailbox => PresentMode::Mailbox,
        false if present_mode_info.supports_immediate => PresentMode::Immediate,
        _ => PresentMode::Fifo,
    };
    Ok(SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format,
        width: width.max(1),
        height: height.max(1),
        desired_maximum_frame_latency: if triple_buffering { 2 } else { 1 },
        present_mode,
        alpha_mode,
        view_formats: vec![],
    })
}

pub struct Surface {
    device: Device,
    surface: wgpu::Surface<'static>,
    config: SurfaceConfiguration,
    present_mode_info: PresentModeInfo,
    invalid: bool,
}

impl Surface {
    pub fn new(
        adapter: &Adapter,
        device: Device,
        surface: wgpu::Surface<'static>,
        window_width: u32,
        window_height: u32,
        triple_buffering: bool,
        vsync: bool,
    ) -> Result<Self, String> {
        let capabilities = surface.get_capabilities(adapter);
        let config = surface_configuration(&capabilities, window_width, window_height, triple_buffering, vsync).map_err(|error| {
            let info = adapter.get_info();
            format!(
                "{error}; adapter={:?}, backend={:?}, formats={:?}, present_modes={:?}",
                info.name, info.backend, capabilities.formats, capabilities.present_modes
            )
        })?;
        let present_mode_info = PresentModeInfo::from_capabilities(&capabilities);

        #[cfg(feature = "debug")]
        {
            print_debug!("Surface present mode is {:?}", config.present_mode.magenta());
            print_debug!("Surface format is {:?}", config.format);
        }

        surface.configure(&device, &config);

        Ok(Self {
            device,
            surface,
            config,
            present_mode_info,
            invalid: false,
        })
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn acquire(&mut self) -> Option<SurfaceTexture> {
        match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(texture) => Some(texture),
            CurrentSurfaceTexture::Suboptimal(texture) => {
                self.invalid = true;
                Some(texture)
            }
            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => None,
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost | CurrentSurfaceTexture::Validation => {
                self.surface.configure(&self.device, &self.config);
                match self.surface.get_current_texture() {
                    CurrentSurfaceTexture::Success(texture) => Some(texture),
                    CurrentSurfaceTexture::Suboptimal(texture) => Some(texture),
                    CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => None,
                    CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost | CurrentSurfaceTexture::Validation => None,
                }
            }
        }
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
        self.config.width = (window_size.width as u32).max(1);
        self.config.height = (window_size.height as u32).max(1);
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

#[cfg(test)]
mod tests {
    use wgpu::CompositeAlphaMode;

    use super::*;

    fn supported() -> SurfaceCapabilities {
        SurfaceCapabilities {
            formats: vec![TextureFormat::Bgra8UnormSrgb],
            present_modes: vec![PresentMode::Fifo],
            alpha_modes: vec![CompositeAlphaMode::Opaque],
            usages: TextureUsages::RENDER_ATTACHMENT,
        }
    }

    #[test]
    fn rejects_surfaceless_adapter_without_formats() {
        let mut caps = supported();
        caps.formats.clear();
        let error = surface_configuration(&caps, 800, 600, false, true).unwrap_err();
        assert!(error.contains("No presentable surface formats"));
        assert!(error.contains("native display"));
    }

    #[test]
    fn rejects_empty_or_incompatible_present_modes() {
        let mut caps = supported();
        caps.present_modes.clear();
        assert!(
            surface_configuration(&caps, 800, 600, false, true)
                .unwrap_err()
                .contains("No presentation modes")
        );
        caps.present_modes.push(PresentMode::Mailbox);
        assert!(surface_configuration(&caps, 800, 600, false, false).unwrap_err().contains("Fifo"));
    }

    #[test]
    fn rejects_missing_render_usage_or_alpha_mode() {
        let mut caps = supported();
        caps.usages = TextureUsages::COPY_DST;
        assert!(
            surface_configuration(&caps, 800, 600, false, true)
                .unwrap_err()
                .contains("RENDER_ATTACHMENT")
        );
        caps.usages = TextureUsages::RENDER_ATTACHMENT;
        caps.alpha_modes.clear();
        assert!(
            surface_configuration(&caps, 800, 600, false, true)
                .unwrap_err()
                .contains("alpha modes")
        );
    }

    #[test]
    fn selects_advertised_format_alpha_and_safe_dimensions() {
        let caps = supported();
        let config = surface_configuration(&caps, 0, 0, true, true).unwrap();
        assert_eq!(config.format, caps.formats[0]);
        assert_eq!(config.alpha_mode, caps.alpha_modes[0]);
        assert_eq!(config.present_mode, PresentMode::Fifo);
        assert_eq!((config.width, config.height), (1, 1));
        assert_eq!(config.desired_maximum_frame_latency, 2);
    }

    #[test]
    fn uses_only_supported_vsync_off_modes_in_preference_order() {
        let mut caps = supported();
        assert_eq!(
            surface_configuration(&caps, 800, 600, false, false).unwrap().present_mode,
            PresentMode::Fifo
        );
        caps.present_modes.push(PresentMode::Immediate);
        assert_eq!(
            surface_configuration(&caps, 800, 600, false, false).unwrap().present_mode,
            PresentMode::Immediate
        );
        caps.present_modes.push(PresentMode::Mailbox);
        assert_eq!(
            surface_configuration(&caps, 800, 600, false, false).unwrap().present_mode,
            PresentMode::Mailbox
        );
        assert_eq!(
            surface_configuration(&caps, 800, 600, false, true).unwrap().present_mode,
            PresentMode::Fifo
        );
    }
}
