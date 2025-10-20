pub(crate) mod mixer;

use std::sync::Mutex;

use cgmath::{EuclideanSpace, Point3, Quaternion};
use rtrb::{Consumer, Producer, RingBuffer};

use self::mixer::Mixer;
use crate::error::ResourceLimitReached;
use crate::listener::{Listener, ListenerHandle};
use crate::manager::Capacities;
use crate::track::{MainTrackBuilder, MainTrackHandle, Track};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
pub(crate) struct ResourceKey(pub(crate) usize);

pub(crate) struct ResourceStorage<T> {
    pub(crate) resources: Vec<Option<T>>,
    new_resource_consumer: Consumer<(ResourceKey, T)>,
    freed_key_producer: Producer<ResourceKey>,
}

impl<T> ResourceStorage<T> {
    #[must_use]
    pub(crate) fn new(capacity: usize) -> (Self, ResourceController<T>) {
        let (new_resource_producer, new_resource_consumer) = RingBuffer::new(capacity);
        let (mut freed_key_producer, freed_key_consumer) = RingBuffer::new(capacity);

        let mut resources = Vec::with_capacity(capacity);

        for key in 0..capacity {
            freed_key_producer
                .push(ResourceKey(key))
                .expect("created more keys than capacity available");

            resources.push(None);
        }

        (
            Self {
                resources,
                new_resource_consumer,
                freed_key_producer,
            },
            ResourceController {
                new_resource_producer: Mutex::new(new_resource_producer),
                freed_key_consumer: Mutex::new(freed_key_consumer),
            },
        )
    }

    pub(crate) fn remove_and_add(&mut self, mut remove_test: impl FnMut(&T) -> bool) {
        // Remove items matching the predicate and recycle their keys.
        for (index, slot) in self.resources.iter_mut().enumerate() {
            if let Some(resource) = slot
                && remove_test(resource)
            {
                slot.take();
                let key = ResourceKey(index);
                self.freed_key_producer
                    .push(key)
                    .unwrap_or_else(|_| panic!("freed key producer is full"));
            }
        }

        while let Ok((key, resource)) = self.new_resource_consumer.pop() {
            self.resources[key.0] = Some(resource);
        }
    }

    #[must_use]
    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.resources.iter().filter_map(|opt| opt.as_ref())
    }

    #[must_use]
    pub(crate) fn is_empty(&self) -> bool {
        self.resources.iter().all(|slot| slot.is_none())
    }
}

impl<'a, T> IntoIterator for &'a mut ResourceStorage<T> {
    type IntoIter = std::iter::FilterMap<std::slice::IterMut<'a, Option<T>>, fn(&'a mut Option<T>) -> Option<&'a mut T>>;
    type Item = &'a mut T;

    fn into_iter(self) -> Self::IntoIter {
        self.resources.iter_mut().filter_map(|opt| opt.as_mut())
    }
}

pub(crate) struct ResourceController<T> {
    pub(crate) new_resource_producer: Mutex<Producer<(ResourceKey, T)>>,
    freed_key_consumer: Mutex<Consumer<ResourceKey>>,
}

impl<T> ResourceController<T> {
    pub(crate) fn insert(&mut self, resource: T) -> Result<ResourceKey, ResourceLimitReached> {
        let key = self.try_reserve()?;
        self.insert_with_key(key, resource);
        Ok(key)
    }

    pub(crate) fn try_reserve(&self) -> Result<ResourceKey, ResourceLimitReached> {
        match self.freed_key_consumer.lock().unwrap().pop() {
            Ok(recycled_key) => Ok(recycled_key),
            _ => Err(ResourceLimitReached),
        }
    }

    pub(crate) fn insert_with_key(&mut self, key: ResourceKey, resource: T) {
        self.new_resource_producer
            .get_mut()
            .expect("new resource producer mutex poisoned")
            .push((key, resource))
            .unwrap_or_else(|_| panic!("new resource producer full"));
    }
}

pub(crate) struct Resources {
    pub(crate) mixer: Mixer,
    pub(crate) listener: Listener,
}

pub(crate) struct ResourceControllers {
    pub(crate) sub_track_controller: ResourceController<Track>,
    pub(crate) main_track_handle: MainTrackHandle,
    pub(crate) listener_handle: ListenerHandle,
}

pub(crate) fn create_resources(
    capacities: Capacities,
    main_track_builder: MainTrackBuilder,
    internal_buffer_size: usize,
) -> (Resources, ResourceControllers) {
    let (mixer, sub_track_controller, main_track_handle) =
        Mixer::new(capacities.sub_track_capacity, internal_buffer_size, main_track_builder);

    let (listener, listener_handle) = Listener::new(Point3::origin(), Quaternion::new(1.0, 0.0, 0.0, 0.0));

    (Resources { mixer, listener }, ResourceControllers {
        sub_track_controller,
        main_track_handle,
        listener_handle,
    })
}
