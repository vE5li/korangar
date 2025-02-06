use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Point3, Transform, Vector2, Vector3, Zero};

use super::Camera;
use crate::graphics::orthographic_reverse_lh;

const FAR_PLANE: f32 = 500.0;
const NEAR_PLANE: f32 = -500.0;
const MAX_BOUNDS: f32 = 200.0;
const FOCUS_POINT_OFFSET: f32 = 50.0;
const ORIGIN: Point3<f32> = Point3::new(0.0, 0.0, 0.0);
const LOOK_UP: Vector3<f32> = Vector3::new(0.0, 1.0, 0.0);

pub struct DirectionalShadowCamera {
    focus_point: Point3<f32>,
    camera_position: Point3<f32>,
    view_direction: Vector3<f32>,
    main_camera_view_direction: Vector3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    view_projection_matrix: Matrix4<f32>,
}

impl DirectionalShadowCamera {
    pub fn new() -> Self {
        Self {
            focus_point: ORIGIN,
            camera_position: ORIGIN,
            view_direction: Vector3::zero(),
            main_camera_view_direction: Vector3::zero(),
            view_matrix: Matrix4::zero(),
            projection_matrix: Matrix4::zero(),
            view_projection_matrix: Matrix4::zero(),
        }
    }

    /// Offsets the focus point to account for the angled view.
    pub fn set_focus_point(&mut self, focus_point: Point3<f32>, view_direction: Vector3<f32>) {
        let planar_direction = Vector3::new(view_direction.x, 0.0, view_direction.z).normalize();
        self.focus_point = focus_point + planar_direction * FOCUS_POINT_OFFSET;
    }

    pub fn update(&mut self, direction_to_light: Vector3<f32>, main_camera_view_direction: Vector3<f32>, shadow_map_size: u32) {
        // TODO: NHA Currently the directional light is the direction TO the light.
        //       We should change that to make it the direction the light shines.
        let direction_to_light = direction_to_light.normalize();

        self.focus_point = self.snap_to_texel(self.focus_point, shadow_map_size, direction_to_light);
        self.camera_position = self.focus_point + direction_to_light * 100.0;
        self.view_direction = -direction_to_light;
        self.main_camera_view_direction = main_camera_view_direction;
    }

    fn snap_to_texel(&mut self, point: Point3<f32>, shadow_map_size: u32, direction_to_light: Vector3<f32>) -> Point3<f32> {
        let units_per_texel = shadow_map_size as f32 / (MAX_BOUNDS * 2.0);
        let texel_scaling_matrix = Matrix4::from_scale(units_per_texel);

        let center = Point3::from_vec(-direction_to_light);
        let view_matrix = Matrix4::look_at_lh(ORIGIN, center, LOOK_UP) * texel_scaling_matrix;

        let inverse_view_matrix = view_matrix.inverse_transform().unwrap();
        let point = view_matrix.transform_point(point);
        let snapped_point = Point3::new(point.x.floor(), point.y.floor(), point.z);
        inverse_view_matrix.transform_point(snapped_point)
    }
}

impl Camera for DirectionalShadowCamera {
    fn camera_position(&self) -> Point3<f32> {
        self.camera_position
    }

    fn focus_point(&self) -> Point3<f32> {
        self.focus_point
    }

    fn generate_view_projection(&mut self, _texture_size: Vector2<usize>) {
        self.view_matrix = Matrix4::look_at_lh(self.camera_position(), self.focus_point, LOOK_UP);
        self.projection_matrix = orthographic_reverse_lh(-MAX_BOUNDS, MAX_BOUNDS, -MAX_BOUNDS, MAX_BOUNDS, NEAR_PLANE, FAR_PLANE);
        self.view_projection_matrix = self.projection_matrix * self.view_matrix;
    }

    fn look_up_vector(&self) -> Vector3<f32> {
        LOOK_UP
    }

    fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
        (self.view_matrix, self.projection_matrix)
    }

    fn view_projection_matrix(&self) -> Matrix4<f32> {
        self.view_projection_matrix
    }

    fn view_direction(&self) -> Vector3<f32> {
        self.view_direction
    }
}
