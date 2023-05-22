use std::net::Ipv4Addr;

use cgmath::{Matrix3, Quaternion, Vector2, Vector3, Vector4};

#[const_trait]
pub trait FixedByteSizeWrapper {
    fn size_in_bytes() -> usize;
}

impl<T: ~const FixedByteSize> const FixedByteSizeWrapper for Vec<T> {
    fn size_in_bytes() -> usize {
        T::size_in_bytes()
    }
}

#[const_trait]
pub trait FixedByteSize {
    fn size_in_bytes() -> usize;
}

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
        core::mem::size_of::<T>() * SIZE
    }
}

impl<T: ~const FixedByteSize> const FixedByteSize for Vector2<T> {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<T>() * 2
    }
}

impl<T: ~const FixedByteSize> const FixedByteSize for Vector3<T> {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<T>() * 3
    }
}

impl<T: ~const FixedByteSize> const FixedByteSize for Vector4<T> {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<T>() * 4
    }
}

impl<T: ~const FixedByteSize> const FixedByteSize for Quaternion<T> {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<T>() * 4
    }
}

impl<T: ~const FixedByteSize> const FixedByteSize for Matrix3<T> {
    fn size_in_bytes() -> usize {
        core::mem::size_of::<T>() * 9
    }
}

impl const FixedByteSize for Ipv4Addr {
    fn size_in_bytes() -> usize {
        4
    }
}
