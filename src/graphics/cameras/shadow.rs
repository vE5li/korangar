use std::sync::Arc;
use cgmath::EuclideanSpace;

use crate::types::maths::*;

use crate::graphics::Transform;
use crate::types::Entity;
use crate::types::map::Map;
use super::RenderSettings;

use super::{ Camera, SmoothedValue };

const ZOOM_SPEED: f32 = 4.0;
const ROTATION_SPEED: f32 = 0.02;

pub struct ShadowCamera {
    focus_position: Point3<f32>,
    look_up_vector: Vector3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    world_to_screen_matrix: Matrix4<f32>,
    screen_to_world_matrix: Matrix4<f32>,
    day_timer: f32,
}

impl ShadowCamera {

    pub fn new() -> Self {
        Self {
            focus_position: Point3::new(0.0, 0.0, 0.0),
            look_up_vector: Vector3::new(0.0, -1.0, 0.0),
            view_matrix: Matrix4::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            projection_matrix: Matrix4::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            world_to_screen_matrix: Matrix4::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            screen_to_world_matrix: Matrix4::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            day_timer: 0.0,
        }
    }

    pub fn set_focus_point(&mut self, position: Vector3<f32>) {
        self.focus_position = Point3::new(position.x, position.y, position.z);
    }

    pub fn update(&mut self, day_timer: f32) {
        self.day_timer = day_timer;
    }

    fn camera_position(&self) -> Point3<f32> {
        let direction = crate::types::map::get_light_direction(self.day_timer).normalize();
        let scaled_direction = direction * 100.0;
        self.focus_position + scaled_direction
    }

    fn view_direction(&self) -> Vector3<f32> {
        let camera_position = self.camera_position();
        Vector3::new(self.focus_position.x - camera_position.x, self.focus_position.y - camera_position.y, self.focus_position.z - camera_position.z).normalize()  
    }

    fn world_to_clip_space(&self, world_space_position: Vector3<f32>) -> Vector4<f32> {
        let position = Vector4::new(world_space_position.x, world_space_position.y, world_space_position.z, 1.0);
        self.world_to_screen_matrix * position  
    }

    fn clip_to_screen_space(&self, clip_space_position: Vector4<f32>) -> Vector2<f32> {
        Vector2::new(clip_space_position.x / clip_space_position.w + 1.0, clip_space_position.y / clip_space_position.w + 1.0)  
    }
}

impl Camera for ShadowCamera {

    fn generate_view_projection(&mut self, _window_size: Vector2<usize>) {

        let bounds = vector4!(-300.0, 300.0, -300.0, 300.0);
        let z_near = -1000.0;
        let z_far = 500.0;

        self.projection_matrix = cgmath::ortho(bounds.x, bounds.y, bounds.w, bounds.z, z_near, z_far);
        self.view_matrix = Matrix4::look_at_rh(self.camera_position(), self.focus_position, self.look_up_vector);
        self.world_to_screen_matrix = self.projection_matrix * self.view_matrix;
        self.screen_to_world_matrix = self.world_to_screen_matrix.invert().unwrap();
    }

    fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
        (self.view_matrix, self.projection_matrix)  
    }

    fn transform_matrix(&self, transform: &Transform) -> Matrix4<f32> {
        let translation_matrix = Matrix4::from_translation(transform.position);
        let rotation_matrix = Matrix4::from_angle_x(transform.rotation.x) * Matrix4::from_angle_y(transform.rotation.y) * Matrix4::from_angle_z(transform.rotation.z);
        let scale_matrix = Matrix4::from_nonuniform_scale(transform.scale.x, transform.scale.y, transform.scale.z);

        translation_matrix * rotation_matrix * scale_matrix 
    }

    fn billboard_matrix(&self, position: Vector3<f32>, origin: Vector3<f32>, size: Vector2<f32>) -> Matrix4<f32> {

        let direction = self.view_direction();
        let right_vector = self.look_up_vector.cross(direction).normalize();
        let up_vector = direction.cross(right_vector).normalize();

        let rotation_matrix = Matrix4::new(right_vector.x, right_vector.y, right_vector.z, 0.0, up_vector.x, up_vector.y, up_vector.z, 0.0, direction.x, direction.y, direction.z, 0.0, 0.0, 0.0, 0.0, 1.0);
        let translation_matrix = Matrix4::from_translation(position);
        let origin_matrix = Matrix4::from_translation(origin);
        let scale_matrix = Matrix4::from_nonuniform_scale(size.x, size.y, 1.0);

        translation_matrix * (rotation_matrix * origin_matrix) * scale_matrix  
    }

    fn billboard_coordinates(&self, position: Vector3<f32>, size: f32) -> (Vector4<f32>, Vector4<f32>) {

        let view_direction = self.view_direction();
        let right_vector = self.look_up_vector.cross(view_direction).normalize();
        let up_vector = view_direction.cross(right_vector).normalize();

        let top_left_position = self.world_to_clip_space(position + (up_vector - right_vector) * size);
        let bottom_right_position = self.world_to_clip_space(position + (right_vector - up_vector) * size);

        (top_left_position, bottom_right_position)  
    }

    fn screen_position_size(&self, top_left_position: Vector4<f32>, bottom_right_position: Vector4<f32>) -> (Vector2<f32>, Vector2<f32>) {
        let top_left_position = self.clip_to_screen_space(top_left_position);
        let bottom_right_position = self.clip_to_screen_space(bottom_right_position);

        let screen_position = top_left_position;
        let screen_size = bottom_right_position - top_left_position;

        (screen_position, screen_size)  
    }

    fn distance_to(&self, position: Vector3<f32>) -> f32 {
        let delta = self.camera_position() - position;
        delta.map(|component| component * component).sum().sqrt()  
    }

    fn get_screen_to_world_matrix(&self) -> Matrix4<f32> {
        self.screen_to_world_matrix  
    }
}
