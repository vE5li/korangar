use derive_new::new;

use graphics::RenderSettings;
use Entity;

#[derive(new)]
pub struct StateProvider<'t> {
    pub render_settings: &'t RenderSettings,
    pub player: &'t Entity,
}
