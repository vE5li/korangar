use derive_new::new;

use crate::types::maths::*;
use crate::graphics::{ DeferredRenderer, Renderer, Color };
use rand::{ thread_rng, Rng };

pub trait Particle {

    fn update(&mut self, delta_time: f32) -> bool;

    fn render(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, renderer: &DeferredRenderer);
}

#[derive(new)]
pub struct DamageNumber {
    position: Vector2<f32>,
    damage_amount: String,
    #[new(value = "200.0")]
    velocity_y: f32,
    #[new(value = "thread_rng().gen_range(-50.0..50.0)")]
    velocity_x: f32,
    #[new(value = "0.6")]
    timer: f32,
}

impl Particle for DamageNumber {

    fn update(&mut self, delta_time: f32) -> bool {
        self.velocity_y -= 700.0 * delta_time;
        self.position.y -= self.velocity_y * delta_time;
        self.position.x += self.velocity_x * delta_time;
        self.timer -= delta_time;
        self.timer > 0.0
    }

    fn render(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, renderer: &DeferredRenderer) {
        renderer.render_text(render_target, &self.damage_amount, self.position, Color::monochrome(255), 16.0);
    }
}

#[derive(Default)]
pub struct ParticleHolder {
    particles: Vec<Box<dyn Particle + Send + Sync>>,
}

impl ParticleHolder {

    pub fn spawn_particle(&mut self, particle: Box<dyn Particle + Send + Sync>) {
        self.particles.push(particle);
    }

    pub fn update(&mut self, delta_time: f32) {
        self.particles.retain_mut(|particle| particle.update(delta_time));
    }

    pub fn render(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, renderer: &DeferredRenderer) {
        self.particles.iter().for_each(|particle| particle.render(render_target, renderer));
    }
}
