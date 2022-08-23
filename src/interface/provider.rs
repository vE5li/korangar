use derive_new::new;

use crate::{graphics::RenderSettings, network::LoginSettings};

#[derive(new)]
pub struct StateProvider<'t> {
    pub render_settings: &'t RenderSettings,
    pub login_settings: &'t LoginSettings,
}
