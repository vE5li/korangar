use cgmath::{EuclideanSpace, InnerSpace, Matrix4, Point3, Transform, Vector2, Vector3, Zero};
use korangar_util::collision::AABB;

use super::Camera;
use crate::graphics::orthographic_reverse_lh;

const ORIGIN: Point3<f32> = Point3::new(0.0, 0.0, 0.0);
const LOOK_UP: Vector3<f32> = Vector3::new(0.0, 1.0, 0.0);

const CAMERA_NEAR_PLANE: f32 = 0.1;
const CAMERA_FAR_PLANE: f32 = 750.0;

pub struct DirectionalShadowCamera {
    level_bound: AABB,
    camera_position: Point3<f32>,
    view_direction: Vector3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    view_projection_matrix: Matrix4<f32>,
}

impl DirectionalShadowCamera {
    pub fn new() -> Self {
        Self {
            level_bound: AABB::uninitialized(),
            camera_position: ORIGIN,
            view_direction: Vector3::zero(),
            view_matrix: Matrix4::zero(),
            projection_matrix: Matrix4::zero(),
            view_projection_matrix: Matrix4::zero(),
        }
    }

    /// Sets the level bounds. Used to properly bound the near plane of the
    /// directional shadow.
    pub fn set_level_bound(&mut self, level_bound: AABB) {
        self.level_bound = level_bound;
    }

    /// Computes the minimum Z value of the level AABB in light view space.
    fn compute_level_min_z(&self, light_view: &Matrix4<f32>) -> f32 {
        let min = self.level_bound.min();
        let max = self.level_bound.max();

        let corners = [
            Point3::new(min.x, min.y, min.z),
            Point3::new(max.x, min.y, min.z),
            Point3::new(min.x, max.y, min.z),
            Point3::new(max.x, max.y, min.z),
            Point3::new(min.x, min.y, max.z),
            Point3::new(max.x, min.y, max.z),
            Point3::new(min.x, max.y, max.z),
            Point3::new(max.x, max.y, max.z),
        ];

        let mut min_z = f32::MAX;
        for corner in &corners {
            let transformed = light_view.transform_point(*corner);
            min_z = min_z.min(transformed.z);
        }

        min_z
    }

    /// Computes the axis aligned extents of the camera frustum in light space.
    fn compute_frustum_extents(
        camera_view_inv: &Matrix4<f32>,
        camera_proj: &Matrix4<f32>,
        light_view: &Matrix4<f32>,
    ) -> (Vector3<f32>, Vector3<f32>) {
        // Extract inverse scales from projection matrix to compute frustum corners.
        let scale_x_inv = 1.0 / camera_proj.x.x;
        let scale_y_inv = 1.0 / camera_proj.y.y;

        // Transform from camera view space to light view space.
        let camera_view_to_light_view = light_view * camera_view_inv;

        // Compute the 8 corners of the frustum in view space.
        let mut corners = [Point3::origin(); 8];

        // Near plane corners (in view space).
        let near_x = scale_x_inv * CAMERA_NEAR_PLANE;
        let near_y = scale_y_inv * CAMERA_NEAR_PLANE;
        corners[0] = Point3::new(-near_x, near_y, CAMERA_NEAR_PLANE);
        corners[1] = Point3::new(near_x, near_y, CAMERA_NEAR_PLANE);
        corners[2] = Point3::new(-near_x, -near_y, CAMERA_NEAR_PLANE);
        corners[3] = Point3::new(near_x, -near_y, CAMERA_NEAR_PLANE);

        // Far plane corners (in view space).
        let far_x = scale_x_inv * CAMERA_FAR_PLANE;
        let far_y = scale_y_inv * CAMERA_FAR_PLANE;
        corners[4] = Point3::new(-far_x, far_y, CAMERA_FAR_PLANE);
        corners[5] = Point3::new(far_x, far_y, CAMERA_FAR_PLANE);
        corners[6] = Point3::new(-far_x, -far_y, CAMERA_FAR_PLANE);
        corners[7] = Point3::new(far_x, -far_y, CAMERA_FAR_PLANE);

        // Transform corners to light view space and compute AABB.
        let mut min_corner = Vector3::new(f32::MAX, f32::MAX, f32::MAX);
        let mut max_corner = Vector3::new(f32::MIN, f32::MIN, f32::MIN);

        for corner in &corners {
            let transformed = camera_view_to_light_view.transform_point(*corner);
            min_corner.x = min_corner.x.min(transformed.x);
            min_corner.y = min_corner.y.min(transformed.y);
            min_corner.z = min_corner.z.min(transformed.z);
            max_corner.x = max_corner.x.max(transformed.x);
            max_corner.y = max_corner.y.max(transformed.y);
            max_corner.z = max_corner.z.max(transformed.z);
        }

        (min_corner, max_corner)
    }

    pub fn update_from_camera(
        &mut self,
        direction_to_light: Vector3<f32>,
        main_camera_view: &Matrix4<f32>,
        main_camera_proj: &Matrix4<f32>,
    ) {
        let direction_to_light = direction_to_light.normalize();
        self.view_direction = -direction_to_light;

        let camera_view_inv = main_camera_view
            .inverse_transform()
            .expect("Camera view matrix should be invertible");

        // Look from the light position.
        let eye = Point3::from_vec(direction_to_light);
        let at = ORIGIN;
        let up = if direction_to_light.y.abs() > 0.99 {
            Vector3::new(1.0, 0.0, 0.0)
        } else {
            Vector3::new(0.0, 1.0, 0.0)
        };

        let mut light_view = Matrix4::look_at_lh(eye, at, up);

        let (min, max) = Self::compute_frustum_extents(&camera_view_inv, main_camera_proj, &light_view);

        // Center the light view on the frustum extents.
        let center = (min + max) * 0.5;
        let center_transform = Matrix4::from_translation(Vector3::new(-center.x, -center.y, -min.z));
        light_view = center_transform * light_view;

        // Create projection matrix that covers the extents.
        let dimensions = max - min;

        // Check if there is level geometry that we need to cover.
        let level_min_z = self.compute_level_min_z(&light_view);
        let near_plane = if level_min_z < 0.0 { level_min_z } else { 0.0 };

        let light_projection = orthographic_reverse_lh(
            -dimensions.x * 0.5,
            dimensions.x * 0.5,
            -dimensions.y * 0.5,
            dimensions.y * 0.5,
            near_plane,
            dimensions.z,
        );

        self.view_matrix = light_view;
        self.projection_matrix = light_projection;
        self.view_projection_matrix = light_projection * light_view;

        let view_inv = light_view.inverse_transform().expect("Light view matrix should be invertible");
        self.camera_position = Point3::from_vec(view_inv.w.truncate());
    }
}

impl Camera for DirectionalShadowCamera {
    fn camera_position(&self) -> Point3<f32> {
        self.camera_position
    }

    fn focus_point(&self) -> Point3<f32> {
        unimplemented!()
    }

    fn generate_view_projection(&mut self, _texture_size: Vector2<usize>) {
        unimplemented!()
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
