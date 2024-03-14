use cgmath::{Array, EuclideanSpace, InnerSpace, Matrix4, MetricSpace, Point3, SquareMatrix, Vector2, Vector3, Vector4};

use super::Camera;
use crate::graphics::Transform;
use crate::interface::layout::{ScreenPosition, ScreenSize};

pub struct ShadowCamera {
    focus_point: Point3<f32>,
    look_up_vector: Vector3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    world_to_screen_matrix: Matrix4<f32>,
    screen_to_world_matrix: Matrix4<f32>,
    day_timer: f32,
}

impl ShadowCamera {
    const FAR_PLANE: f32 = 500.0;
    const NEAR_PLANE: f32 = -1000.0;

    pub fn new() -> Self {
        Self {
            focus_point: Point3::new(0.0, 0.0, 0.0),
            look_up_vector: Vector3::new(0.0, -1.0, 0.0),
            view_matrix: Matrix4::from_value(0.0),
            projection_matrix: Matrix4::from_value(0.0),
            world_to_screen_matrix: Matrix4::from_value(0.0),
            screen_to_world_matrix: Matrix4::from_value(0.0),
            day_timer: 0.0,
        }
    }

    pub fn set_focus_point(&mut self, focus_point: Point3<f32>) {
        self.focus_point = focus_point;
    }

    pub fn update(&mut self, day_timer: f32) {
        self.day_timer = day_timer;
    }

    fn camera_position(&self) -> Point3<f32> {
        let direction = crate::world::get_light_direction(self.day_timer).normalize();
        let scaled_direction = direction * 100.0;
        self.focus_point + scaled_direction
    }

    fn view_direction(&self) -> Vector3<f32> {
        let camera_position = self.camera_position();
        Vector3::new(
            self.focus_point.x - camera_position.x,
            self.focus_point.y - camera_position.y,
            self.focus_point.z - camera_position.z,
        )
        .normalize()
    }

    fn world_to_clip_space(&self, world_space_position: Vector3<f32>) -> Vector4<f32> {
        let position = Vector4::new(world_space_position.x, world_space_position.y, world_space_position.z, 1.0);
        self.world_to_screen_matrix * position
    }

    fn clip_to_screen_space(&self, clip_space_position: Vector4<f32>) -> Vector2<f32> {
        Vector2::new(
            clip_space_position.x / clip_space_position.w + 1.0,
            clip_space_position.y / clip_space_position.w + 1.0,
        )
    }
}

impl Camera for ShadowCamera {
    fn generate_view_projection(&mut self, _window_size: Vector2<usize>) {
        let bounds = Vector4::new(-300.0, 300.0, -300.0, 300.0);

        self.projection_matrix = cgmath::ortho(bounds.x, bounds.y, bounds.w, bounds.z, Self::NEAR_PLANE, Self::FAR_PLANE);
        self.view_matrix = Matrix4::look_at_rh(self.camera_position(), self.focus_point, self.look_up_vector);
        self.world_to_screen_matrix = self.projection_matrix * self.view_matrix;
        self.screen_to_world_matrix = self.world_to_screen_matrix.invert().unwrap();
    }

    fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
        (self.view_matrix, self.projection_matrix)
    }

    fn transform_matrix(&self, transform: &Transform) -> Matrix4<f32> {
        let translation_matrix = Matrix4::from_translation(transform.position);
        let rotation_matrix = Matrix4::from_angle_x(transform.rotation.x)
            * Matrix4::from_angle_y(transform.rotation.y)
            * Matrix4::from_angle_z(transform.rotation.z);
        let scale_matrix = Matrix4::from_nonuniform_scale(transform.scale.x, transform.scale.y, transform.scale.z);

        translation_matrix * rotation_matrix * scale_matrix
    }

    fn billboard_matrix(&self, position: Vector3<f32>, origin: Vector3<f32>, size: Vector2<f32>) -> Matrix4<f32> {
        let direction = self.view_direction();
        let right_vector = self.look_up_vector.cross(direction).normalize();
        let up_vector = direction.cross(right_vector).normalize();

        let rotation_matrix = Matrix4::from_cols(
            right_vector.extend(0.0),
            up_vector.extend(0.0),
            direction.extend(0.0),
            Vector3::from_value(0.0).extend(1.0),
        );

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

    fn screen_position_size(&self, top_left_position: Vector4<f32>, bottom_right_position: Vector4<f32>) -> (ScreenPosition, ScreenSize) {
        let top_left_position = self.clip_to_screen_space(top_left_position);
        let bottom_right_position = self.clip_to_screen_space(bottom_right_position);

        let screen_position = ScreenPosition {
            left: top_left_position.x,
            top: top_left_position.y,
        };
        let screen_size = ScreenSize {
            width: bottom_right_position.x - top_left_position.x,
            height: bottom_right_position.y - top_left_position.y,
        };

        (screen_position, screen_size)
    }

    fn distance_to(&self, position: Vector3<f32>) -> f32 {
        self.camera_position().distance(Point3::from_vec(position))
    }

    fn get_screen_to_world_matrix(&self) -> Matrix4<f32> {
        self.screen_to_world_matrix
    }

    fn get_camera_direction(&self) -> usize {
        let view_direction = self.view_direction();
        super::direction(Vector2::new(view_direction.x, view_direction.z))
    }

    fn calculate_depth_offset_and_curvature(&self, world_matrix: &Matrix4<f32>) -> (f32, f32) {
        let zero_point = world_matrix * Vector4::new(0.0, 0.0, 0.0, 1.0);
        let top_point = world_matrix * Vector4::new(0.0, -2.0, 0.0, 1.0);
        let visual_length = zero_point.distance(top_point);
        let visual_top_point = zero_point + Vector4::new(0.0, visual_length, 0.0, 0.0);

        let top_depth = (self.world_to_screen_matrix * top_point).z;
        let visual_top_depth = (self.world_to_screen_matrix * visual_top_point).z;

        let depth_offset = visual_top_depth - top_depth;
        // TODO: derive this somehow to make this code generic. Probaly zfar - znear /
        // magic
        let curvature = 0.003;

        (depth_offset, curvature)
    }
}
