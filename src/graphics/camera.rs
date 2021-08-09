use cgmath::{ Matrix4, Vector3, Point3, Rad, Angle };
use std::f32::consts::FRAC_PI_2;
use vertex_shader::ty::Matrices;

use graphics::Transform;

pub struct Camera {
    look_at: Point3<f32>,
    camera: Point3<f32>,
    up: Vector3<f32>,
    view_matrix: Matrix4<f32>,
    projection_matrix: Matrix4<f32>,
}

impl Camera {

    pub fn new() -> Self {
        Self {
            look_at: Point3::new(0.0, 0.0, 0.0),
            camera: Point3::new(0.0, 3.5, 3.5),
            up: Vector3::new(0.0, 0.0, 1.0),
            view_matrix: Matrix4::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            projection_matrix: Matrix4::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        }
    }

    fn rotation_matrix(rotation: Vector3<Rad<f32>>) -> Matrix4<f32> {
        let sx = rotation.x.sin();
        let cx = rotation.x.cos();
        let sy = rotation.y.sin();
        let cy = rotation.y.cos();
        let sz = rotation.z.sin();
        let cz = rotation.z.cos();

        Matrix4::new( // z
             cz, -sz, 0.0, 0.0,
             sz,  cz, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ) * Matrix4::new( // y
             cy, 0.0,  sy, 0.0,
            0.0, 1.0, 0.0, 0.0,
            -sy, 0.0,  cy, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ) * Matrix4::new( // x
            1.0, 0.0, 0.0, 0.0,
            0.0,  cx, -sx, 0.0,
            0.0,  sx,  cx, 0.0,
            0.0, 0.0, 0.0, 1.0,
        )
    }

    pub fn generate_view_projection(&mut self, dimensions: [u32; 2]) {
        let aspect_ratio = dimensions[0] as f32 / dimensions[1] as f32;
        self.projection_matrix = cgmath::perspective(Rad(FRAC_PI_2), aspect_ratio, 0.01, 100.0);
        self.view_matrix = Matrix4::look_at_rh(self.camera, self.look_at, self.up) * Matrix4::from_scale(1.0);
    }

    pub fn matrix_buffer_data(&self, transform: &Transform) -> Matrices {
        let translation_matrix = Matrix4::from_translation(transform.position);
        let rotation_matrix = Self::rotation_matrix(transform.rotation);
        let scale_matrix = Matrix4::from_nonuniform_scale(transform.scale.x, transform.scale.y, transform.scale.z);
        let transform_matrix = rotation_matrix * translation_matrix * scale_matrix;

        return Matrices {
            world: transform_matrix.into(),
            view: self.view_matrix.into(),
            projection: self.projection_matrix.into(),
        };
    }
}
