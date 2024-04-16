use std::ops::Add;

use cgmath::{Deg, Rad, Vector3};
use korangar_interface::elements::PrototypeElement;
use ragnarok_bytes::{ByteStream, ConversionResult, ConversionResultExt, FromBytes};

#[derive(Copy, Clone, Debug, PrototypeElement)]
pub struct Transform {
    pub position: Vector3<f32>,
    #[hidden_element] // TODO: unhide
    pub rotation: Vector3<Rad<f32>>,
    pub scale: Vector3<f32>,
}

impl FromBytes for Transform {
    fn from_bytes<META>(byte_stream: &mut ByteStream<META>) -> ConversionResult<Self> {
        let mut position = <Vector3<f32>>::from_bytes(byte_stream).trace::<Self>()?;
        let rotation = <Vector3<f32>>::from_bytes(byte_stream).trace::<Self>()?;
        let scale = <Vector3<f32>>::from_bytes(byte_stream).trace::<Self>()?;

        // TODO: make this nicer
        position.y = -position.y;

        Ok(Transform::from(position, rotation.map(Deg), scale))
    }
}

impl Transform {
    pub fn from(position: Vector3<f32>, rotation: Vector3<Deg<f32>>, scale: Vector3<f32>) -> Self {
        let rotation = rotation.map(|degrees| degrees.into());
        Self { position, rotation, scale }
    }

    pub fn position(position: Vector3<f32>) -> Self {
        Self {
            position,
            rotation: Vector3::new(Rad(0.0), Rad(0.0), Rad(0.0)),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}

impl Add for Transform {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            position: self.position + other.position,
            rotation: Vector3::new(
                self.rotation.x + other.rotation.x,
                self.rotation.y + other.rotation.y,
                self.rotation.z + other.rotation.z,
            ),
            scale: Vector3::new(
                self.scale.x * other.scale.x,
                self.scale.y * other.scale.y,
                self.scale.z * other.scale.z,
            ),
        }
    }
}
