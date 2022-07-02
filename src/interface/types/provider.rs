use derive_new::new;

use graphics::RenderSettings;


#[derive(new)]
pub struct StateProvider<'t> {
    pub render_settings: &'t RenderSettings,
    //pub player: &'t Entity,
}
