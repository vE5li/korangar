use cgmath::{ Vector3, Rad };
use std::ops::Add;

#[derive(Copy, Clone, Debug)]
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

    pub fn from(position: Vector3<f32>, rotation: Vector3<Rad<f32>>, scale: Vector3<f32>) -> Self {
        return Self { position, rotation, scale };
    }

    pub fn position(position: Vector3<f32>) -> Self {
        return Self {
            position: position,
            rotation: Vector3::new(Rad(0.0), Rad(0.0), Rad(0.0)),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    pub fn rotation_around_axis(_axis: Vector3<f32>, _angle: Rad<f32>) -> Self {
        return Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Vector3::new(Rad(0.0), Rad(0.0), Rad(0.0)), // implement actual rotation
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    //pub fn rotation(rotation: Vector3<Rad<f32>>) -> Self {
    //    return Self {
    //        position: Vector3::new(0.0, 0.0, 0.0),
    //        rotation: rotation,
    //        scale: Vector3::new(1.0, 1.0, 1.0),
    //    }
    //}

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
            scale: Vector3::new(self.scale.x * other.scale.x, self.scale.y * other.scale.y, self.scale.z * other.scale.z),
        }
    }
}
