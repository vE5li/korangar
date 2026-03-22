//! The main entrypoint for controlling audio from gameplay code.
//!
//! In order to play audio, you'll need to create an [`AudioManager`].
//! The [`AudioManager`] keeps track of playing sounds and manages other
//! resources like mixer tracks and spatial scenes. Once the
//! [`AudioManager`] is dropped, its audio output will be stopped.

use std::sync::Arc;

use crate::backend::resources::{ResourceControllers, create_resources};
use crate::backend::{Backend, DefaultBackend, Renderer};
use crate::device_info::{DeviceId, OutputDevicePreference};
use crate::error::ResourceLimitReached;
use crate::listener::ListenerHandle;
use crate::track::{MainTrackBuilder, MainTrackHandle, TrackBuilder, TrackHandle};

/// Size of the internal mixing buffer in frames.
const INTERNAL_BUFFER_SIZE: usize = 256;

/// Controls audio from gameplay code.
pub(crate) struct AudioManager<B: Backend = DefaultBackend> {
    _backend: B,
    resource_controllers: ResourceControllers,
    preference: Arc<OutputDevicePreference>,
}

impl<B: Backend> AudioManager<B> {
    /// Creates a new [`AudioManager`].
    pub(crate) fn new(settings: AudioManagerSettings, preferred: Option<DeviceId>) -> Result<Self, B::Error> {
        let (mut backend, _device_info, preference) = B::setup(preferred)?;

        let (resources, resource_controllers) =
            create_resources(settings.capacities, settings.main_track_builder, INTERNAL_BUFFER_SIZE);

        let renderer = Renderer::new(resources);

        backend.start(renderer)?;
        Ok(Self {
            _backend: backend,
            resource_controllers,
            preference,
        })
    }

    /// Creates a mixer sub-track.
    pub(crate) fn add_sub_track(&mut self, builder: TrackBuilder) -> Result<TrackHandle, ResourceLimitReached> {
        let (track, handle) = builder.build(INTERNAL_BUFFER_SIZE);
        self.resource_controllers.sub_track_controller.insert(track)?;
        Ok(handle)
    }

    /// Returns the spatial listener handle.
    #[must_use]
    pub(crate) fn listener(&self) -> &ListenerHandle {
        &self.resource_controllers.listener_handle
    }

    /// Returns a handle to the main mixer track.
    #[must_use]
    pub(crate) fn main_track(&mut self) -> &mut MainTrackHandle {
        &mut self.resource_controllers.main_track_handle
    }

    /// Returns the device preference.
    pub(crate) fn preference(&self) -> &Arc<OutputDevicePreference> {
        &self.preference
    }
}

/// Specifies how many of each resource type an audio context can have.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Capacities {
    pub(crate) sub_track_capacity: usize,
}

impl Default for Capacities {
    fn default() -> Self {
        Self { sub_track_capacity: 64 }
    }
}

/// Settings for an [`AudioManager`](AudioManager).
pub(crate) struct AudioManagerSettings {
    pub(crate) capacities: Capacities,
    pub(crate) main_track_builder: MainTrackBuilder,
}

impl Default for AudioManagerSettings {
    fn default() -> Self {
        Self {
            capacities: Capacities::default(),
            main_track_builder: MainTrackBuilder::default(),
        }
    }
}
