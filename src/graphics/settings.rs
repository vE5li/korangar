use derive_new::new;
use procedural::toggle;

#[derive(toggle, new)]
pub struct GraphicsSettings {
    #[toggle]
    #[new(value = "true")]
    pub frame_limit: bool,
    #[toggle]
    #[new(value = "true")]
    pub show_interface: bool,
}
