use cgmath::{ Matrix4, Vector3, Point3, Rad, SquareMatrix };
use std::f32::consts::FRAC_PI_2;
use graphics::{ Matrices, Transform, SmoothedValue };

pub struct Camera {
    look_at: Point3<f32>,
    up: Vector3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
    zoom: SmoothedValue,
    view_angle: SmoothedValue,
}

impl Camera {

    pub fn new() -> Self {
        Self {
            look_at: Point3::new(0.0, 0.0, 0.0),
            up: Vector3::new(0.0, -1.0, 0.0),
            view_matrix: Matrix4::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            projection_matrix: Matrix4::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            zoom: SmoothedValue::new(30.0, 0.01, 15.0),
            view_angle: SmoothedValue::new(0.0, 0.01, 5.0),
        }
    }

    pub fn update(&mut self, delta_time: f64) {
        self.zoom.update(delta_time);
        self.view_angle.update(delta_time);
    }

    pub fn generate_view_projection(&mut self, dimensions: [u32; 2]) {
        let aspect_ratio = dimensions[0] as f32 / dimensions[1] as f32;
        self.projection_matrix = cgmath::perspective(Rad(FRAC_PI_2), aspect_ratio, 0.01, 1000.0);

        let zoom = self.zoom.get_current();
        let view_angle = self.view_angle.get_current();
        let camera = Point3::new(zoom * view_angle.cos(), zoom, -zoom * view_angle.sin());

        self.view_matrix = Matrix4::look_at_rh(camera, self.look_at, self.up) * Matrix4::from_scale(1.0);
    }

    pub fn screen_to_world_matrix(&self) -> Matrix4<f32> {
        return (self.projection_matrix * self.view_matrix).invert().unwrap();
    }

    pub fn matrix_buffer_data(&self, transform: &Transform) -> Matrices {
        let translation_matrix = Matrix4::from_translation(transform.position);
        let rotation_matrix = Matrix4::from_angle_x(transform.rotation.x) * Matrix4::from_angle_y(transform.rotation.y) * Matrix4::from_angle_z(transform.rotation.z);
        let scale_matrix = Matrix4::from_nonuniform_scale(transform.scale.x, transform.scale.y, transform.scale.z);
        let transform_matrix = rotation_matrix * translation_matrix * scale_matrix;

        return Matrices {
            rotation: rotation_matrix.into(),
            world: transform_matrix.into(),
            view: self.view_matrix.into(),
            projection: self.projection_matrix.into(),
        };
    }

    pub fn soft_zoom(&mut self, zoom_factor: f32) {
        self.zoom.move_desired(zoom_factor);
        // clamp selection
    }

    pub fn soft_rotate(&mut self, rotation: f32) {
        self.view_angle.move_desired(rotation);
    }
}
