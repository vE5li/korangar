use cgmath::{ Vector3, Rad };
use std::ops::Add;

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

    pub fn scale(scale: Vector3<f32>) -> Self {
        return Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(Rad(0.0), Rad(0.0), Rad(0.0)),
            scale: scale,
        }
    }
}

impl Add for Transform {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            position: self.position + other.position,
            rotation: Vector3::new(self.rotation.x + other.rotation.x, self.rotation.y + other.rotation.y, self.rotation.z + other.rotation.z),
            scale: self.scale + other.scale,
        }
    }
}
