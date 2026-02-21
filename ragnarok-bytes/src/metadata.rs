use std::any::{Any, TypeId};

/// Caster trait used to convert from `&dyn Any` to `&dyn T`.
pub struct Caster<T: ?Sized> {
    /// Cast any to an immutable reference.
    cast_ref: fn(&dyn Any) -> Option<&T>,

    /// Cast any to a mutable reference.
    cast_mut: fn(&mut dyn Any) -> Option<&mut T>,
}

impl<T: ?Sized> Caster<T> {
    /// Creates a new caster from `&dyn Any` to `&dyn T` and `&mut dyn Any` to
    /// `&mut dyn T`.
    pub fn new(cast_ref: fn(&dyn Any) -> Option<&T>, cast_mut: fn(&mut dyn Any) -> Option<&mut T>) -> Self {
        Self { cast_ref, cast_mut }
    }
}

struct CasterEntry {
    trait_id: TypeId,
    caster: Box<dyn Any>,
}

/// Metadata storage using dynamic dispatch.
pub struct DynMetadata {
    data: Box<dyn Any>,
    // We typically expect no more that 1-2 casters, so the overhead of iterating the vector to
    // find the correct entry is smaller than the overhead of something like a `HashMap`.
    caster_entries: Vec<CasterEntry>,
}

impl DynMetadata {
    /// Creates new dynamic metadata from static metadata.
    pub(super) fn new<T: 'static>(data: T) -> Self {
        Self {
            data: Box::new(data),
            caster_entries: Vec::new(),
        }
    }

    /// Register a caster, thereby enabling the metadata to be cast into `T`.
    pub fn register_caster<T: ?Sized + 'static>(&mut self, caster: Caster<T>) {
        self.caster_entries.push(CasterEntry {
            trait_id: TypeId::of::<T>(),
            caster: Box::new(caster),
        });
    }

    /// Try to get the metadata as `&T`.
    pub(super) fn get<T: ?Sized + 'static>(&self) -> Option<&T> {
        let target_trait_id = TypeId::of::<T>();
        let caster_any = self
            .caster_entries
            .iter()
            .find(|entry| entry.trait_id == target_trait_id)
            .map(|entry| &entry.caster)?;

        let caster = caster_any.downcast_ref::<Caster<T>>()?;

        (caster.cast_ref)(&*self.data)
    }

    /// Try to get the metadata as `&mut T`.
    pub(super) fn get_mut<T: ?Sized + 'static>(&mut self) -> Option<&mut T> {
        let target_trait_id = TypeId::of::<T>();
        let caster_any = self
            .caster_entries
            .iter_mut()
            .find(|entry| entry.trait_id == target_trait_id)
            .map(|entry| &mut entry.caster)?;

        let caster = caster_any.downcast_mut::<Caster<T>>()?;

        (caster.cast_mut)(&mut *self.data)
    }
}

/// Trait for metadata that can be used by the
/// [`ByteReader`](super::ByteReader).
pub trait CastableMetadata: 'static {
    /// Registers all [`Caster`]s for trait conversions supported by `Self`.
    fn register(metadata: &mut DynMetadata);
}
