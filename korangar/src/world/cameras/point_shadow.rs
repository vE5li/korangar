use std::f32::consts::SQRT_2;

use cgmath::{Deg, InnerSpace, Matrix4, Point3, Vector2, Vector3, Zero};

use super::Camera;
use crate::graphics::{ScreenSize, perspective_reverse_lh};

const VERTICAL_FOV: Deg<f32> = Deg(90.0);

pub struct PointShadowCamera {
    camera_position: Point3<f32>,
    view_direction: Vector3<f32>,
    look_up_vector: Vector3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    view_projection_matrix: Matrix4<f32>,
}

impl PointShadowCamera {
    pub fn new() -> Self {
        Self {
            camera_position: Point3::new(0.0, 0.0, 0.0),
            view_direction: Vector3::unit_x(),
            look_up_vector: Vector3::unit_y(),
            view_matrix: Matrix4::zero(),
            projection_matrix: Matrix4::zero(),
            view_projection_matrix: Matrix4::zero(),
        }
    }

    pub fn set_camera_position(&mut self, camera_position: Point3<f32>) {
        self.camera_position = camera_position;
    }

    pub fn change_direction(&mut self, direction: u32) {
        (self.view_direction, self.look_up_vector) = match direction {
            0 => (Vector3::unit_x(), Vector3::unit_y()),
            1 => (-Vector3::unit_x(), Vector3::unit_y()),
            2 => (Vector3::unit_y(), Vector3::unit_z()),
            3 => (-Vector3::unit_y(), Vector3::unit_z()),
            4 => (Vector3::unit_z(), Vector3::unit_y()),
            5 => (-Vector3::unit_z(), Vector3::unit_y()),
            _ => panic!(),
        };
    }

    /// Checks if a sphere intersects the frustum of the current cube face.
    /// Only the four side planes are tested, since the range of the light is
    /// already culled separately.
    pub fn sphere_intersects_face(&self, center: Point3<f32>, radius: f32) -> bool {
        let relative = center - self.camera_position;
        let right_vector = self.look_up_vector.cross(self.view_direction);

        let forward = relative.dot(self.view_direction);
        let horizontal = relative.dot(right_vector).abs();
        let vertical = relative.dot(self.look_up_vector).abs();

        // With a 90 degree FOV the side planes have normals at 45 degrees, so the
        // signed distance to a plane is `(forward - side) / sqrt(2)`.
        let limit = -radius * SQRT_2;

        forward - horizontal >= limit && forward - vertical >= limit
    }
}

/// Camera used to render entities into a single face of a point shadow map.
///
/// Entity billboards are oriented towards the light source, while the view
/// and projection matrices of the current cube face are kept, so that the
/// depth offsets match the face being rendered. Since the billboard
/// orientation does not depend on the face, entities that span multiple faces
/// are rendered consistently without seams.
pub struct PointShadowEntityCamera<'a> {
    face_camera: &'a PointShadowCamera,
    view_direction: Vector3<f32>,
}

impl<'a> PointShadowEntityCamera<'a> {
    pub fn new(face_camera: &'a PointShadowCamera, target: Point3<f32>) -> Self {
        // Keep a minimal horizontal component, so that the billboard and sprite
        // direction are well defined even if the light is directly above or
        // below the entity.
        const MINIMUM_HORIZONTAL_LENGTH: f32 = 0.01;

        let offset = target - face_camera.camera_position;

        let mut view_direction = match offset.magnitude2() > f32::EPSILON {
            true => offset.normalize(),
            false => -Vector3::unit_y(),
        };

        let horizontal = Vector2::new(view_direction.x, view_direction.z);
        let horizontal_length = horizontal.magnitude();

        if horizontal_length < MINIMUM_HORIZONTAL_LENGTH {
            let horizontal_direction = match horizontal_length > f32::EPSILON {
                true => horizontal / horizontal_length,
                false => Vector2::unit_y(),
            };

            view_direction = Vector3::new(
                horizontal_direction.x * MINIMUM_HORIZONTAL_LENGTH,
                view_direction.y,
                horizontal_direction.y * MINIMUM_HORIZONTAL_LENGTH,
            )
            .normalize();
        }

        Self {
            face_camera,
            view_direction,
        }
    }
}

impl Camera for PointShadowEntityCamera<'_> {
    fn camera_position(&self) -> Point3<f32> {
        self.face_camera.camera_position
    }

    fn focus_point(&self) -> Point3<f32> {
        self.face_camera.camera_position + self.view_direction
    }

    fn generate_view_projection(&mut self, _window_size: ScreenSize) {}

    fn look_up_vector(&self) -> Vector3<f32> {
        Vector3::unit_y()
    }

    fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
        self.face_camera.view_projection_matrices()
    }

    fn view_projection_matrix(&self) -> Matrix4<f32> {
        self.face_camera.view_projection_matrix()
    }

    fn view_direction(&self) -> Vector3<f32> {
        self.view_direction
    }
}

impl Camera for PointShadowCamera {
    fn camera_position(&self) -> Point3<f32> {
        self.camera_position
    }

    fn focus_point(&self) -> Point3<f32> {
        self.camera_position + self.view_direction
    }

    fn generate_view_projection(&mut self, _window_size: ScreenSize) {
        self.view_matrix = Matrix4::look_to_lh(self.camera_position, self.view_direction, self.look_up_vector);
        self.projection_matrix = perspective_reverse_lh(VERTICAL_FOV, 1.0);
        self.view_projection_matrix = self.projection_matrix * self.view_matrix;
    }

    fn look_up_vector(&self) -> Vector3<f32> {
        self.look_up_vector
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
