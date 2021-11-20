mod smoothed;
mod player;

#[cfg(feature = "debug")]
mod debug;

pub use self::player::PlayerCamera;

#[cfg(feature = "debug")]
pub use self::debug::DebugCamera;

use self::smoothed::SmoothedValue;

use cgmath::{ Matrix4, Vector4, Vector3, Vector2 };
use graphics::Transform;

pub trait Camera {

    fn generate_view_projection(&mut self, window_size: Vector2<usize>);

    fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>);

    fn transform_matrix(&self, transform: &Transform) -> (Matrix4<f32>, Matrix4<f32>);

    fn billboard_matrix(&self, position: Vector3<f32>, origin: Vector3<f32>, size: Vector2<f32>) -> Matrix4<f32>;

    fn billboard_coordinates(&self, position: Vector3<f32>, size: f32) -> (Vector4<f32>, Vector4<f32>);

    fn screen_position_size(&self, top_left_position: Vector4<f32>, bottom_right_position: Vector4<f32>) -> (Vector2<f32>, Vector2<f32>);

    fn distance_to(&self, position: Vector3<f32>) -> f32;

    fn get_screen_to_world_matrix(&self) -> Matrix4<f32>;
}

/*impl Camera {

    fn generate_view_projection(&mut self, window_size: Vector2<usize>) {
        let aspect_ratio = window_size.x as f32 / window_size.y as f32;
        self.projection_matrix = cgmath::perspective(Rad(0.2617), aspect_ratio, 1.0, 10000.0);
        self.view_matrix = Matrix4::look_at_rh(self.camera_position(), self.focus_position, self.look_up_vector);
        self.world_to_screen_matrix = self.projection_matrix * self.view_matrix;
        self.screen_to_world_matrix = self.world_to_screen_matrix.invert().unwrap();
    }

    fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
        return (self.view_matrix, self.projection_matrix);
    }

    fn transform_matrix(&self, transform: &Transform) -> (Matrix4<f32>, Matrix4<f32>) {
        let translation_matrix = Matrix4::from_translation(transform.position);
        let rotation_matrix = Matrix4::from_angle_x(transform.rotation.x) * Matrix4::from_angle_y(transform.rotation.y) * Matrix4::from_angle_z(transform.rotation.z);
        let scale_matrix = Matrix4::from_nonuniform_scale(transform.scale.x, transform.scale.y, transform.scale.z);
        let world_matrix = rotation_matrix * translation_matrix * scale_matrix;

        return (rotation_matrix, world_matrix);
    }

    fn billboard_matrix(&self, position: Vector3<f32>, size: Vector2<f32>) -> Matrix4<f32> {
        let point = Point3::new(position.x, position.y, position.z);
        let translation_matrix = Matrix4::from_translation(position);
        let rotation_matrix = Matrix4::look_at_rh(point, self.camera_position(), Vector3::new(0.0, -1.0, 0.0));
        let scale_matrix = Matrix4::from_nonuniform_scale(size.x, size.y, 1.0);
        let world_matrix = rotation_matrix * translation_matrix * scale_matrix;

        return world_matrix;
    }

    fn billboard_coordinates(&self, position: Vector3<f32>, size: f32) -> (Vector4<f32>, Vector4<f32>) {

        let view_direction = self.view_direction();
        let right_vector = self.look_up_vector.cross(view_direction).normalize();
        let up_vector = view_direction.cross(right_vector).normalize();

        let top_left_position = self.world_to_clip_space(position + (up_vector - right_vector) * size);
        let bottom_right_position = self.world_to_clip_space(position + (right_vector - up_vector) * size);

        return (top_left_position, bottom_right_position);
    }

    fn screen_position_size(&self, top_left_position: Vector4<f32>, bottom_right_position: Vector4<f32>) -> (Vector2<f32>, Vector2<f32>) {
        let top_left_position = self.clip_to_screen_space(top_left_position);
        let bottom_right_position = self.clip_to_screen_space(bottom_right_position);

        let screen_position = top_left_position;
        let screen_size = bottom_right_position - top_left_position;

        return (screen_position, screen_size);
    }

    fn distance_to(&self, position: Vector3<f32>) -> f32 {
        let delta = self.camera_position() - position;
        return delta.map(|component| component * component).sum().sqrt();
    }

    fn get_screen_to_world_matrix(&self) -> Matrix4<f32> {
        return self.screen_to_world_matrix;
    }
}*/
