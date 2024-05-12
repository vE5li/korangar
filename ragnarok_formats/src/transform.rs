use std::ops::Add;

use cgmath::{Deg, Rad, Vector3};
use ragnarok_bytes::{ByteStream, ConversionResult, ConversionResultExt, FromBytes, ToBytes};

#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct Transform {
    pub position: Vector3<f32>,
    #[cfg_attr(feature = "interface", hidden_element)] // TODO: unhide
    pub rotation: Vector3<Rad<f32>>,
    pub scale: Vector3<f32>,
}

impl FromBytes for Transform {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let mut position = <Vector3<f32>>::from_bytes(byte_stream).trace::<Self>()?;
        let rotation = <Vector3<f32>>::from_bytes(byte_stream).trace::<Self>()?;
        let scale = <Vector3<f32>>::from_bytes(byte_stream).trace::<Self>()?;

        // Convert from a standard Rust float (which is in degrees) to a stronger cgmath
        // type that also represents degrees. We can then easily convert it to
        // radians.
        let rotation = rotation.map(|degrees| Deg(degrees).into());

        // TODO: make this nicer
        position.y = -position.y;

        Ok(Transform { position, rotation, scale })
    }
}

impl ToBytes for Transform {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        let position = Vector3::new(self.position.x, -self.position.y, self.position.z);
        let rotation = self.rotation.map(|radiants| Deg::from(radiants).0);
        let scale = self.scale;

        let mut bytes = position.to_bytes().trace::<Self>()?;
        bytes.extend(rotation.to_bytes().trace::<Self>()?);
        bytes.extend(scale.to_bytes().trace::<Self>()?);

        Ok(bytes)
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

#[cfg(test)]
mod conversion {
    use ragnarok_bytes::{ByteStream, FromBytes, ToBytes};

    use super::Transform;

    #[test]
    fn transform() {
        let input = &[
            1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 1, 2, 3, 0, 4, 5, 6, 0, 7, 8, 9, 10, 0, 0, 0, 11, 0, 0, 0, 12, 0, 0, 0,
        ];
        let mut byte_stream = ByteStream::<()>::without_metadata(input);

        let transform = Transform::from_bytes(&mut byte_stream).unwrap();
        let output = transform.to_bytes().unwrap();

        assert_eq!(input, output.as_slice());
    }
}
