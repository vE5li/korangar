mod graphics;
mod audio;
#[cfg(feature = "debug")]
mod render;

pub use self::graphics::GraphicsSettingsWindow;
pub use self::audio::AudioSettingsWindow;
#[cfg(feature = "debug")]
pub use self::render::RenderSettingsWindow;
