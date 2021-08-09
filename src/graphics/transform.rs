use cgmath::{ Vector3, Rad };

pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: Vector3<Rad<f32>>,
    pub scale: Vector3<f32>,
}

impl Transform {

    pub fn new() -> Self {
        return Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(Rad(0.0), Rad(0.0), Rad(0.0)),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    pub fn from() -> Self {
        return Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(Rad(0.0), Rad(0.0), Rad(0.0)),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    pub fn position(position: Vector3<f32>) -> Self {
        return Self {
            position: position,
            rotation: Vector3::new(Rad(0.0), Rad(0.0), Rad(0.0)),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    pub fn rotation(rotation: Vector3<Rad<f32>>) -> Self {
        return Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: rotation,
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}
