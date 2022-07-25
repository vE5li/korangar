use procedural::*;
mod particle;

use derive_new::new;

use crate::graphics::{ Renderer, Camera, Color, DeferredRenderer, MarkerRenderer };
use crate::types::maths::*;

pub use self::particle::Particle;

#[derive(PrototypeElement, PrototypeWindow, new)]
#[window_title("Effect Source")]
pub struct EffectSource {
    pub name: String,
    pub position: Vector3<f32>,
    pub effect_type: usize, // TODO: fix this
    pub emit_speed: f32,
    #[hidden_element]
    #[new(default)]
    pub particles: Vec<Particle>,
    #[hidden_element]
    #[new(default)]
    pub spawn_timer: f32,
}

impl EffectSource {

    pub fn offset(&mut self, offset: Vector3<f32>) {
        self.position += offset;
    }

    pub fn update(&self, delta_time: f32) {

        let mut_self = unsafe { &mut *(self as *const Self as *mut Self) };
        mut_self.spawn_timer += delta_time;

        if mut_self.spawn_timer > 0.3 {
            mut_self.particles.push(Particle::new(self.position, Color::rgb(255, 50, 50), 10.0));
            mut_self.spawn_timer -= 1.0;
        }

        let mut index = 0;
        while index < self.particles.len() {
            match mut_self.particles[index].update(delta_time) {
                true => index += 1,
                false => { mut_self.particles.remove(index); },
            }
        }
    }

    pub fn render_lights(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, renderer: &DeferredRenderer, camera: &dyn Camera) {
        self.particles.iter().for_each(|particle| renderer.point_light(render_target, camera, particle.position, particle.light_color, particle.light_range));
    }

    #[cfg(feature = "debug")]
    pub fn render_marker<T>(&self, render_target: &mut T::Target, renderer: &T, camera: &dyn Camera, hovered: bool)
        where T: Renderer + MarkerRenderer
    {
        renderer.render_marker(render_target, camera, self.position, hovered);
    }
}
