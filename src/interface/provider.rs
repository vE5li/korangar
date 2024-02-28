use derive_new::new;

use crate::graphics::GraphicsSettings;
#[cfg(feature = "debug")]
use crate::graphics::RenderSettings;

#[derive(new)]
pub struct StateProvider<'t> {
    pub graphics_settings: &'t GraphicsSettings,
    #[cfg(feature = "debug")]
    pub render_settings: &'t RenderSettings,
}
