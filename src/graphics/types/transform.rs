use cgmath::{ Matrix4, Vector3, Rad, Deg, SquareMatrix };
use std::ops::Add;

#[derive(Copy, Clone, Debug, PrototypeElement)]
pub struct Transform {
    pub position: Vector3<f32>,
    #[hidden_element] // TODO: unhide 
    pub rotation: Vector3<Rad<f32>>,
    pub scale: Vector3<f32>,
    #[hidden_element]
    pub node_translation: Matrix4<f32>,
    #[hidden_element]
    pub offset_translation: Matrix4<f32>,
    #[hidden_element]
    pub offset_matrix: Matrix4<f32>,
    #[hidden_element]
    pub rotation_matrix: Matrix4<f32>,
    #[hidden_element]
    pub node_scale: Matrix4<f32>,
}

impl Transform {

    pub fn new() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(Rad(0.0), Rad(0.0), Rad(0.0)),
            scale: Vector3::new(1.0, 1.0, 1.0),
            node_translation: Matrix4::identity(),
            offset_translation: Matrix4::identity(),
            offset_matrix: Matrix4::identity(),
            rotation_matrix: Matrix4::identity(),
            node_scale: Matrix4::identity(),
        }
    }

    pub fn from(position: Vector3<f32>, rotation: Vector3<Deg<f32>>, scale: Vector3<f32>) -> Self {

        let rotation = rotation.map(|degrees| degrees.into());

        Self { position, rotation, scale, node_translation: Matrix4::identity(), offset_translation: Matrix4::identity(), offset_matrix: Matrix4::identity(), rotation_matrix: Matrix4::identity(), node_scale: Matrix4::identity() }
    }

    pub fn position(position: Vector3<f32>) -> Self {
        Self {
            position,
            rotation: Vector3::new(Rad(0.0), Rad(0.0), Rad(0.0)),
            scale: Vector3::new(1.0, 1.0, 1.0),
            node_translation: Matrix4::identity(),
            offset_translation: Matrix4::identity(),
            offset_matrix: Matrix4::identity(),
            rotation_matrix: Matrix4::identity(),
            node_scale: Matrix4::identity(),
        }
    }

    pub fn node_translation(node_translation: Vector3<f32>) -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(Rad(0.0), Rad(0.0), Rad(0.0)),
            scale: Vector3::new(1.0, 1.0, 1.0),
            node_translation: Matrix4::from_translation(node_translation),
            offset_translation: Matrix4::identity(),
            offset_matrix: Matrix4::identity(),
            rotation_matrix: Matrix4::identity(),
            node_scale: Matrix4::identity(),
        }
    }

    pub fn rotation_around_axis(axis: Vector3<f32>, angle: Rad<f32>) -> Self {

        //let rotation_matrix = Matrix3::from_axis_angle(axis, angle);
        //let x = rotation_matrix[1][2].atan2(rotation_matrix[2][2]);
        //let y = rotation_matrix[0][2].atan2((rotation_matrix[1][2].powf(2.0) + rotation_matrix[2][2].powf(2.0)).sqrt());
        //let z = rotation_matrix[0][1].atan2(rotation_matrix[0][0]);

        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            //rotation: Vector3::new(Rad(x), Rad(y), Rad(z)),
            rotation: Vector3::new(Rad(0.0), Rad(0.0), Rad(0.0)),
            scale: Vector3::new(1.0, 1.0, 1.0),
            node_translation: Matrix4::identity(),
            offset_translation: Matrix4::identity(),
            offset_matrix: Matrix4::identity(),
            rotation_matrix: Matrix4::from_axis_angle(axis, angle),
            //rotation_matrix: Matrix4::from_axis_angle(Vector3::new(-axis.z, axis.x, axis.y), angle),
            node_scale: Matrix4::identity(),
        }
    }

    //pub fn rotation(rotation: Vector3<Rad<f32>>) -> Self {
    //    Self {
    //        position: Vector3::new(0.0, 0.0, 0.0),
    //        rotation: rotation,
    //        scale: Vector3::new(1.0, 1.0, 1.0),
    //    }
    //}

    pub fn scale(scale: Vector3<f32>) -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(Rad(0.0), Rad(0.0), Rad(0.0)),
            scale,
            node_translation: Matrix4::identity(),
            offset_translation: Matrix4::identity(),
            offset_matrix: Matrix4::identity(),
            rotation_matrix: Matrix4::identity(),
            node_scale: Matrix4::identity(),
        }
    }

    pub fn node_scale(scale: Vector3<f32>) -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(Rad(0.0), Rad(0.0), Rad(0.0)),
            scale: Vector3::new(1.0, 1.0, 1.0),
            node_translation: Matrix4::identity(),
            offset_translation: Matrix4::identity(),
            offset_matrix: Matrix4::identity(),
            rotation_matrix: Matrix4::identity(),
            node_scale: Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z),
        }
    }

    pub fn offset(offset: Vector3<f32>) -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(Rad(0.0), Rad(0.0), Rad(0.0)),
            scale: Vector3::new(1.0, 1.0, 1.0),
            node_translation: Matrix4::identity(),
            offset_translation: Matrix4::from_translation(offset),
            offset_matrix: Matrix4::identity(),
            rotation_matrix: Matrix4::identity(),
            node_scale: Matrix4::identity(),
        }
    }

    pub fn offset_matrix(offset_matrix: Matrix4<f32>) -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(Rad(0.0), Rad(0.0), Rad(0.0)),
            scale: Vector3::new(1.0, 1.0, 1.0),
            node_translation: Matrix4::identity(),
            offset_translation: Matrix4::identity(),
            offset_matrix,
            rotation_matrix: Matrix4::identity(),
            node_scale: Matrix4::identity(),
        }
    }
}

impl Add for Transform {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            position: self.position + other.position,
            rotation: Vector3::new(self.rotation.x + other.rotation.x, self.rotation.y + other.rotation.y, self.rotation.z + other.rotation.z),
            scale: Vector3::new(self.scale.x * other.scale.x, self.scale.y * other.scale.y, self.scale.z * other.scale.z),
            node_translation: self.node_translation * other.node_translation,
            offset_translation: self.offset_translation * other.offset_translation,
            offset_matrix: self.offset_matrix * other.offset_matrix,
            rotation_matrix: self.rotation_matrix * other.rotation_matrix,
            node_scale: self.node_scale * other.node_scale,
        }
    }
}
