mod particle;

use cgmath::{ Vector3, Vector2 };

use graphics::{ Renderer, Camera, Color };

use self::particle::Particle;

pub struct EffectSource {
    position: Vector3<f32>,
    particles: Vec<Particle>,
    spawn_timer: f32,
}

impl EffectSource {

    pub fn new(position: Vector3<f32>) -> Self {

        let particles = Vec::new();
        let spawn_timer = 0.0;

        return Self { position, particles, spawn_timer };
    }

    pub fn offset(&mut self, offset: Vector3<f32>) {
        self.position += offset;
    }

    pub fn update(&self, delta_time: f32) {

        let mut_self = unsafe { &mut *(self as *const Self as *mut Self) };
        mut_self.spawn_timer += delta_time;

        if mut_self.spawn_timer > 0.3 {
            mut_self.particles.push(Particle::new(self.position, Color::new(255, 50, 50), 50.0));
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

    pub fn render_lights(&self, renderer: &mut Renderer, camera: &dyn Camera) {
        self.particles.iter().for_each(|particle| renderer.point_light(camera, particle.position, particle.light_color, particle.light_range));
    }

    #[cfg(feature = "debug")]
    pub fn hovered(&self, renderer: &Renderer, camera: &dyn Camera, mouse_position: Vector2<f32>, smallest_distance: f32) -> Option<f32> {
        let distance = camera.distance_to(self.position);

        match distance < smallest_distance && renderer.marker_hovered(camera, self.position, mouse_position) {
            true => return Some(distance),
            false => return None,
        }
    }

    #[cfg(feature = "debug")]
    pub fn particle_hovered(&self, renderer: &Renderer, camera: &dyn Camera, mouse_position: Vector2<f32>, mut smallest_distance: f32) -> Option<(f32, usize)> {
        let mut closest_particle = None;

        for (index, particle) in self.particles.iter().enumerate() {
            let distance = camera.distance_to(particle.position);

            if distance < smallest_distance && renderer.marker_hovered(camera, particle.position, mouse_position) {
                smallest_distance = distance;
                closest_particle = Some((distance, index));
            }
        }

        return closest_particle;
    }

    #[cfg(feature = "debug")]
    pub fn render_marker(&self, renderer: &mut Renderer, camera: &dyn Camera) {
        renderer.render_effect_marker(camera, self.position);
    }

    #[cfg(feature = "debug")]
    pub fn render_particle_markers(&self, renderer: &mut Renderer, camera: &dyn Camera) {
        self.particles.iter().for_each(|particle| renderer.render_particle_marker(camera, particle.position));
    }
}
