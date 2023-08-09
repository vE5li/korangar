use std::f32::consts::FRAC_PI_2;

use cgmath::{Array, EuclideanSpace, InnerSpace, Matrix4, MetricSpace, Point3, Rad, SquareMatrix, Vector2, Vector3, Vector4};

use super::{Camera, SmoothedValue, ENTITY_CURVATURE_FRACTION};
use crate::graphics::Transform;

const ZOOM_SPEED: f32 = 2.0;
const ROTATION_SPEED: f32 = 0.02;
const MINIMUM_ZOOM: f32 = 150.0;
const MAXIMUM_ZOOM: f32 = 600.0;
const DEFAULT_ZOOM: f32 = 400.0;
const THRESHHOLD: f32 = 0.01;

pub struct PlayerCamera {
    focus_point: Point3<SmoothedValue>,
    look_up_vector: Vector3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    world_to_screen_matrix: Matrix4<f32>,
    screen_to_world_matrix: Matrix4<f32>,
    view_angle: SmoothedValue,
    zoom: SmoothedValue,
    aspect_ratio: f32,
}

impl PlayerCamera {
    const FAR_PLANE: f32 = 2000.0;
    const NEAR_PLANE: f32 = 1.0;

    pub fn new() -> Self {
        Self {
            focus_point: [SmoothedValue::new(0.0, THRESHHOLD, 5.0); 3].into(),
            look_up_vector: Vector3::new(0.0, -1.0, 0.0),
            view_matrix: Matrix4::from_value(0.0),
            projection_matrix: Matrix4::from_value(0.0),
            world_to_screen_matrix: Matrix4::from_value(0.0),
            screen_to_world_matrix: Matrix4::from_value(0.0),
            view_angle: SmoothedValue::new(FRAC_PI_2, THRESHHOLD, 15.0),
            zoom: SmoothedValue::new(DEFAULT_ZOOM, THRESHHOLD, 5.0),
            aspect_ratio: 0.0,
        }
    }

    pub fn get_focus_point(&mut self) -> Point3<f32> {
        self.focus_point.map(|component| component.get_current())
    }

    pub fn set_focus_point(&mut self, position: Vector3<f32>) {
        self.focus_point.x.set(position.x);
        self.focus_point.y.set(position.y);
        self.focus_point.z.set(position.z);
    }

    pub fn set_smoothed_focus_point(&mut self, position: Vector3<f32>) {
        self.focus_point.x.set_desired(position.x);
        self.focus_point.y.set_desired(position.y);
        self.focus_point.z.set_desired(position.z);
    }

    pub fn soft_zoom(&mut self, zoom_factor: f32) {
        self.zoom.move_desired_clamp(zoom_factor * ZOOM_SPEED, MINIMUM_ZOOM, MAXIMUM_ZOOM);
    }

    pub fn soft_rotate(&mut self, rotation: f32) {
        self.view_angle.move_desired(rotation * ROTATION_SPEED);
    }

    pub fn update(&mut self, delta_time: f64) {
        self.focus_point.x.update(delta_time);
        self.focus_point.y.update(delta_time);
        self.focus_point.z.update(delta_time);
        self.zoom.update(delta_time);
        self.view_angle.update(delta_time);
    }

    fn camera_position(&self) -> Point3<f32> {
        let zoom = self.zoom.get_current();
        let view_angle = self.view_angle.get_current();
        Point3::new(
            self.focus_point.x.get_current() + zoom * view_angle.cos(),
            self.focus_point.y.get_current() + zoom,
            self.focus_point.z.get_current() + -zoom * view_angle.sin(),
        )
    }

    fn view_direction(&self) -> Vector3<f32> {
        let camera_position = self.camera_position();
        Vector3::new(
            self.focus_point.x.get_current() - camera_position.x,
            self.focus_point.y.get_current() - camera_position.y,
            self.focus_point.z.get_current() - camera_position.z,
        )
        .normalize()
    }

    fn world_to_clip_space(&self, world_space_position: Vector3<f32>) -> Vector4<f32> {
        self.world_to_screen_matrix * world_space_position.extend(1.0)
    }

    fn clip_to_screen_space(&self, clip_space_position: Vector4<f32>) -> Vector2<f32> {
        Vector2::new(
            clip_space_position.x / clip_space_position.w + 1.0,
            clip_space_position.y / clip_space_position.w + 1.0,
        )
    }
}

impl Camera for PlayerCamera {
    fn generate_view_projection(&mut self, window_size: Vector2<usize>) {
        self.aspect_ratio = window_size.x as f32 / window_size.y as f32;
        self.projection_matrix = cgmath::perspective(Rad(0.2617), self.aspect_ratio, Self::NEAR_PLANE, Self::FAR_PLANE);

        let camera_position = self.camera_position();
        self.view_matrix = Matrix4::look_at_rh(camera_position, self.get_focus_point(), self.look_up_vector);

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

    fn screen_position_size(&self, top_left_position: Vector4<f32>, bottom_right_position: Vector4<f32>) -> (Vector2<f32>, Vector2<f32>) {
        let top_left_position = self.clip_to_screen_space(top_left_position);
        let bottom_right_position = self.clip_to_screen_space(bottom_right_position);

        let screen_position = top_left_position;
        let screen_size = bottom_right_position - top_left_position;

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

        let linear_to_non_linear = |linear_depth: f32| {
            (2.0 * Self::FAR_PLANE * Self::NEAR_PLANE)
                / (Self::FAR_PLANE + Self::NEAR_PLANE - linear_depth * (Self::FAR_PLANE - Self::NEAR_PLANE))
        };

        let top_depth = linear_to_non_linear((self.world_to_screen_matrix * top_point).z);
        let visual_top_depth = linear_to_non_linear((self.world_to_screen_matrix * visual_top_point).z);

        let curvature = visual_top_depth / ENTITY_CURVATURE_FRACTION;
        let depth_offset = visual_top_depth - top_depth;

        (depth_offset, curvature)
    }
}
