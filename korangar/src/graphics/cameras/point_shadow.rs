use cgmath::{Deg, Matrix4, Point3, SquareMatrix, Vector2, Vector3};

use super::Camera;

pub struct PointShadowCamera {
    camera_position: Point3<f32>,
    view_direction: Vector3<f32>,
    look_up_vector: Vector3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    world_to_screen_matrix: Matrix4<f32>,
    screen_to_world_matrix: Matrix4<f32>,
}

impl PointShadowCamera {
    const FAR_PLANE: f32 = 256.0;
    const NEAR_PLANE: f32 = 0.1;

    pub fn new() -> Self {
        Self {
            camera_position: Point3::new(0.0, 0.0, 0.0),
            view_direction: Vector3::new(1.0, 0.0, 0.0),
            look_up_vector: Vector3::new(0.0, -1.0, 0.0),
            view_matrix: Matrix4::from_value(0.0),
            projection_matrix: Matrix4::from_value(0.0),
            world_to_screen_matrix: Matrix4::from_value(0.0),
            screen_to_world_matrix: Matrix4::from_value(0.0),
        }
    }

    pub fn set_camera_position(&mut self, camera_position: Point3<f32>) {
        self.camera_position = camera_position;
    }

    pub fn change_direction(&mut self, direction: u32) {
        (self.view_direction, self.look_up_vector) = match direction {
            0 => (Vector3::new(-1.0, 0.0, 0.0), Vector3::new(0.0, -1.0, 0.0)),
            1 => (Vector3::new(1.0, 0.0, 0.0), Vector3::new(0.0, -1.0, 0.0)),
            2 => (Vector3::new(0.0, 1.0, 0.0), Vector3::new(0.0, 0.0, -1.0)),
            3 => (Vector3::new(0.0, -1.0, 0.0), Vector3::new(0.0, 0.0, -1.0)),
            4 => (Vector3::new(0.0, 0.0, -1.0), Vector3::new(0.0, -1.0, 0.0)),
            5 => (Vector3::new(0.0, 0.0, 1.0), Vector3::new(0.0, -1.0, 0.0)),
            _ => panic!(),
        };
    }
}

impl Camera for PointShadowCamera {
    fn camera_position(&self) -> Point3<f32> {
        self.camera_position
    }

    fn focus_point(&self) -> Point3<f32> {
        self.camera_position + self.view_direction
    }

    fn generate_view_projection(&mut self, _window_size: Vector2<usize>) {
        self.projection_matrix = cgmath::perspective(Deg(90.0), 1.0, Self::NEAR_PLANE, Self::FAR_PLANE);

        self.view_matrix = Matrix4::look_at_lh(self.camera_position, self.focus_point(), self.look_up_vector);

        self.world_to_screen_matrix = self.projection_matrix * self.view_matrix;
        self.screen_to_world_matrix = self.world_to_screen_matrix.invert().unwrap();
    }

    fn look_up_vector(&self) -> Vector3<f32> {
        self.look_up_vector
    }

    fn screen_to_world_matrix(&self) -> Matrix4<f32> {
        self.screen_to_world_matrix
    }

    fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
        (self.view_matrix, self.projection_matrix)
    }

    fn world_to_screen_matrix(&self) -> Matrix4<f32> {
        self.world_to_screen_matrix
    }

    fn view_direction(&self) -> Vector3<f32> {
        self.view_direction
    }
}
