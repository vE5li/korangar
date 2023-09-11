use std::net::Ipv4Addr;

use cgmath::{Matrix3, Quaternion, Vector2, Vector3, Vector4};

#[const_trait]
pub trait FixedByteSizeWrapper {
    fn size_in_bytes(length_hint: Option<usize>) -> usize;
}

impl<T: ~const FixedByteSize> const FixedByteSizeWrapper for Vec<T> {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        T::size_in_bytes(length_hint)
    }
}

#[const_trait]
pub trait FixedByteSize {
    fn size_in_bytes(length_hint: Option<usize>) -> usize;
}

impl const FixedByteSize for u8 {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        assert!(length_hint.is_none());
        core::mem::size_of::<Self>()
    }
}

impl const FixedByteSize for u16 {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        assert!(length_hint.is_none());
        core::mem::size_of::<Self>()
    }
}

impl const FixedByteSize for u32 {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        assert!(length_hint.is_none());
        core::mem::size_of::<Self>()
    }
}

impl const FixedByteSize for u64 {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        assert!(length_hint.is_none());
        core::mem::size_of::<Self>()
    }
}

impl const FixedByteSize for i8 {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        assert!(length_hint.is_none());
        core::mem::size_of::<Self>()
    }
}

impl const FixedByteSize for i16 {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        assert!(length_hint.is_none());
        core::mem::size_of::<Self>()
    }
}

impl const FixedByteSize for i32 {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        assert!(length_hint.is_none());
        core::mem::size_of::<Self>()
    }
}

impl const FixedByteSize for i64 {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        assert!(length_hint.is_none());
        core::mem::size_of::<Self>()
    }
}

impl const FixedByteSize for f32 {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        assert!(length_hint.is_none());
        core::mem::size_of::<Self>()
    }
}

impl const FixedByteSize for f64 {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        assert!(length_hint.is_none());
        core::mem::size_of::<Self>()
    }
}

impl<T: ~const FixedByteSize, const SIZE: usize> const FixedByteSize for [T; SIZE] {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        assert!(length_hint.is_none());
        T::size_in_bytes(None) * SIZE
    }
}

impl<T: ~const FixedByteSize> const FixedByteSize for Vector2<T> {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        assert!(length_hint.is_none());
        T::size_in_bytes(None) * 2
    }
}

impl<T: ~const FixedByteSize> const FixedByteSize for Vector3<T> {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        assert!(length_hint.is_none());
        T::size_in_bytes(None) * 3
    }
}

impl<T: ~const FixedByteSize> const FixedByteSize for Vector4<T> {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        assert!(length_hint.is_none());
        T::size_in_bytes(None) * 4
    }
}

impl<T: ~const FixedByteSize> const FixedByteSize for Quaternion<T> {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        assert!(length_hint.is_none());
        T::size_in_bytes(None) * 4
    }
}

impl<T: ~const FixedByteSize> const FixedByteSize for Matrix3<T> {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        assert!(length_hint.is_none());
        T::size_in_bytes(None) * 9
    }
}

impl const FixedByteSize for Ipv4Addr {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        assert!(length_hint.is_none());
        4
    }
}

impl const FixedByteSize for String {
    fn size_in_bytes(length_hint: Option<usize>) -> usize {
        match length_hint {
            Some(length) => length,
            None => panic!("fixed size string needs to have a length hint"),
        }
    }
}
