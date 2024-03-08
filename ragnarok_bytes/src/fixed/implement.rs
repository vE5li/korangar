use std::net::Ipv4Addr;

#[cfg(feature = "cgmath")]
use cgmath::{Matrix3, Quaternion, Vector2, Vector3, Vector4};

use super::{FixedByteSize, FixedByteSizeWrapper};

impl const FixedByteSize for u8 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl const FixedByteSize for u16 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl const FixedByteSize for u32 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl const FixedByteSize for u64 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl const FixedByteSize for i8 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl const FixedByteSize for i16 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl const FixedByteSize for i32 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl const FixedByteSize for i64 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl const FixedByteSize for f32 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl const FixedByteSize for f64 {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<Self>()
    }
}

impl<T: ~const FixedByteSize, const SIZE: usize> const FixedByteSize for [T; SIZE] {
    fn size_in_bytes() -> usize {
        T::size_in_bytes() * SIZE
    }
}

#[cfg(feature = "cgmath")]
impl<T: ~const FixedByteSize> const FixedByteSize for Vector2<T> {
    fn size_in_bytes() -> usize {
        T::size_in_bytes() * 2
    }
}

#[cfg(feature = "cgmath")]
impl<T: ~const FixedByteSize> const FixedByteSize for Vector3<T> {
    fn size_in_bytes() -> usize {
        T::size_in_bytes() * 3
    }
}

#[cfg(feature = "cgmath")]
impl<T: ~const FixedByteSize> const FixedByteSize for Vector4<T> {
    fn size_in_bytes() -> usize {
        T::size_in_bytes() * 4
    }
}

#[cfg(feature = "cgmath")]
impl<T: ~const FixedByteSize> const FixedByteSize for Quaternion<T> {
    fn size_in_bytes() -> usize {
        T::size_in_bytes() * 4
    }
}

#[cfg(feature = "cgmath")]
impl<T: ~const FixedByteSize> const FixedByteSize for Matrix3<T> {
    fn size_in_bytes() -> usize {
        T::size_in_bytes() * 9
    }
}

#[cfg(feature = "cgmath")]
impl const FixedByteSize for Ipv4Addr {
    fn size_in_bytes() -> usize {
        4
    }
}

impl<T: ~const FixedByteSize> const FixedByteSizeWrapper for Vec<T> {
    fn size_in_bytes() -> usize {
        T::size_in_bytes()
    }
}
