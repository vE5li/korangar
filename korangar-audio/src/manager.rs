//! The main entrypoint for controlling audio from gameplay code.
//!
//! In order to play audio, you'll need to create an [`AudioManager`].
//! The [`AudioManager`] keeps track of playing sounds and manages other
//! resources like mixer tracks and spatial scenes. Once the
//! [`AudioManager`] is dropped, its audio output will be stopped.

use std::sync::Arc;
use std::sync::atomic::AtomicU32;

use crate::backend::resources::{ResourceControllers, create_resources};
use crate::backend::{Backend, DefaultBackend, Renderer};
use crate::device_info::{DeviceId, DeviceInfo, OutputDevicePreference};
use crate::error::ResourceLimitReached;
use crate::listener::ListenerHandle;
use crate::track::{MainTrackBuilder, MainTrackHandle, TrackBuilder, TrackHandle};

/// Size of the internal mixing buffer in frames.
const INTERNAL_BUFFER_SIZE: usize = 256;

/// Controls audio from gameplay code.
pub(crate) struct AudioManager<B: Backend = DefaultBackend> {
    _backend: B,
    resource_controllers: ResourceControllers,
    current_sample_rate: Arc<AtomicU32>,
    preference: Arc<OutputDevicePreference>,
}

impl<B: Backend> AudioManager<B> {
    /// Creates a new [`AudioManager`].
    pub(crate) fn new(settings: AudioManagerSettings, preferred: Option<DeviceId>) -> Result<Self, B::Error> {
        let (mut backend, device_info, preference) = B::setup(preferred)?;
        let current_sample_rate = Arc::new(AtomicU32::new(device_info.sample_rate));

        let (resources, resource_controllers) =
            create_resources(settings.capacities, settings.main_track_builder, INTERNAL_BUFFER_SIZE);

        let renderer = Renderer::new(device_info.sample_rate, INTERNAL_BUFFER_SIZE, resources);

        backend.start(renderer, current_sample_rate.clone())?;
        Ok(Self {
            _backend: backend,
            resource_controllers,
            current_sample_rate,
            preference,
        })
    }

    /// Creates a mixer sub-track.
    pub(crate) fn add_sub_track(&mut self, builder: TrackBuilder) -> Result<TrackHandle, ResourceLimitReached> {
        let (track, handle) = builder.build(self.current_sample_rate.clone(), INTERNAL_BUFFER_SIZE);
        self.resource_controllers.sub_track_controller.insert(track)?;
        Ok(handle)
    }

    /// Returns the spatial listener handle that can be used for updating its
    /// position & orientation.
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

    /// Returns the current live sample rate.
    pub(crate) fn sample_rate(&self) -> u32 {
        self.current_sample_rate.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Specifies how many of each resource type an audio context
/// can have.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Capacities {
    /// The maximum number of mixer sub-tracks that can exist at a time.
    pub(crate) sub_track_capacity: usize,
}

impl Default for Capacities {
    fn default() -> Self {
        Self { sub_track_capacity: 64 }
    }
}

/// Settings for an [`AudioManager`](AudioManager).
pub(crate) struct AudioManagerSettings {
    /// Specifies how many of each resource type an audio context
    /// can have.
    pub(crate) capacities: Capacities,
    /// Configures the main mixer track.
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
