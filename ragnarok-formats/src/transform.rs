use std::ops::Add;

use cgmath::{Deg, EuclideanSpace, Point3, Rad, Vector3};
use ragnarok_bytes::{ByteReader, ByteWriter, ConversionResult, ConversionResultExt, FromBytes, ToBytes};

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct Transform {
    pub position: Point3<f32>,
    #[cfg_attr(feature = "interface", hidden_element)] // TODO: unhide
    pub rotation: Vector3<Rad<f32>>,
    pub scale: Vector3<f32>,
}

impl FromBytes for Transform {
    fn from_bytes<Meta>(byte_reader: &mut ByteReader<Meta>) -> ConversionResult<Self> {
        let mut position = <Point3<f32>>::from_bytes(byte_reader).trace::<Self>()?;
        let rotation = <Vector3<f32>>::from_bytes(byte_reader).trace::<Self>()?;
        let scale = <Vector3<f32>>::from_bytes(byte_reader).trace::<Self>()?;

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
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        byte_writer.write_counted(|write| {
            let position = Vector3::new(self.position.x, -self.position.y, self.position.z);
            let rotation = self.rotation.map(|radiants| Deg::from(radiants).0);
            let scale = self.scale;

            position.to_bytes(write).trace::<Self>()?;
            rotation.to_bytes(write).trace::<Self>()?;
            scale.to_bytes(write).trace::<Self>()?;

            Ok(())
        })
    }
}

impl Transform {
    pub fn from(position: Point3<f32>, rotation: Vector3<Deg<f32>>, scale: Vector3<f32>) -> Self {
        let rotation = rotation.map(|degrees| degrees.into());
        Self { position, rotation, scale }
    }

    pub fn position(position: Point3<f32>) -> Self {
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
            position: self.position + other.position.to_vec(),
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
    use ragnarok_bytes::{ByteReader, ByteWriter, FromBytes, ToBytes};

    use super::Transform;

    #[test]
    fn transform() {
        let input = &[
            1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 0, 1, 2, 3, 0, 4, 5, 6, 0, 7, 8, 9, 10, 0, 0, 0, 11, 0, 0, 0, 12, 0, 0, 0,
        ];
        let mut byte_reader = ByteReader::without_metadata(input);

        let transform = Transform::from_bytes(&mut byte_reader).unwrap();

        let mut byte_writer = ByteWriter::new();
        transform.to_bytes(&mut byte_writer).unwrap();

        assert_eq!(input, byte_writer.into_inner().as_slice());
    }
}
