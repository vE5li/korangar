#[cfg(feature = "cgmath")]
use cgmath::{Matrix3, Point3, Quaternion, Vector2, Vector3, Vector4};

use super::{FixedByteSize, FixedByteSizeCollection};

impl FixedByteSize for u8 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl FixedByteSize for u16 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl FixedByteSize for u32 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl FixedByteSize for u64 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl FixedByteSize for i8 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl FixedByteSize for i16 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl FixedByteSize for i32 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl FixedByteSize for i64 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl FixedByteSize for f32 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl FixedByteSize for f64 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl<T: FixedByteSize, const SIZE: usize> FixedByteSize for [T; SIZE] {
    fn size_in_bytes() -> usize {
        T::size_in_bytes() * SIZE
    }
}

#[cfg(feature = "cgmath")]
impl<T: FixedByteSize> FixedByteSize for Vector2<T> {
    fn size_in_bytes() -> usize {
        T::size_in_bytes() * 2
    }
}

#[cfg(feature = "cgmath")]
impl<T: FixedByteSize> FixedByteSize for Vector3<T> {
    fn size_in_bytes() -> usize {
        T::size_in_bytes() * 3
    }
}

#[cfg(feature = "cgmath")]
impl<T: FixedByteSize> FixedByteSize for Vector4<T> {
    fn size_in_bytes() -> usize {
        T::size_in_bytes() * 4
    }
}

#[cfg(feature = "cgmath")]
impl<T: FixedByteSize> FixedByteSize for Point3<T> {
    fn size_in_bytes() -> usize {
        T::size_in_bytes() * 3
    }
}

#[cfg(feature = "cgmath")]
impl<T: FixedByteSize> FixedByteSize for Quaternion<T> {
    fn size_in_bytes() -> usize {
        T::size_in_bytes() * 4
    }
}

#[cfg(feature = "cgmath")]
impl<T: FixedByteSize> FixedByteSize for Matrix3<T> {
    fn size_in_bytes() -> usize {
        T::size_in_bytes() * 9
    }
}

impl<T: FixedByteSize> FixedByteSizeCollection for Vec<T> {
    fn size_in_bytes() -> usize {
        T::size_in_bytes()
    }
}
