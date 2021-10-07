use cgmath::Vector3;

#[cfg(feature = "debug")]
use graphics::{ Renderer, Camera };

pub struct SoundSource {
    position: Vector3<f32>,
    _range: f32,
}

impl SoundSource {

    pub fn new(position: Vector3<f32>, range: f32) -> Self {
        return Self { position, _range: range };
    }

    pub fn offset(&mut self, offset: Vector3<f32>) {
        self.position += offset;
    }

    #[cfg(feature = "debug")]
    pub fn render_marker(&self, renderer: &mut Renderer, camera: &dyn Camera) {
        renderer.render_sound_icon(camera, self.position);
    }
}
