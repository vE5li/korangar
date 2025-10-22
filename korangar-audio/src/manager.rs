//! The main entrypoint for controlling audio from gameplay code.
//!
//! In order to play audio, you'll need to create an [`AudioManager`].
//! The [`AudioManager`] keeps track of playing sounds and manages other
//! resources like mixer tracks and spatial scenes. Once the
//! [`AudioManager`] is dropped, its audio output will be stopped.

use std::sync::Arc;

use crate::backend::resources::{ResourceControllers, create_resources};
use crate::backend::{Backend, DefaultBackend, Renderer, RendererShared};
use crate::error::ResourceLimitReached;
use crate::listener::ListenerHandle;
use crate::track::{MainTrackBuilder, MainTrackHandle, TrackBuilder, TrackHandle};

/// Controls audio from gameplay code.
pub(crate) struct AudioManager<B: Backend = DefaultBackend> {
    _backend: B,
    resource_controllers: ResourceControllers,
    renderer_shared: Arc<RendererShared>,
    internal_buffer_size: usize,
}

impl<B: Backend> AudioManager<B> {
    /// Creates a new [`AudioManager`].
    pub(crate) fn new(settings: AudioManagerSettings) -> Result<Self, B::Error> {
        let (mut backend, sample_rate) = B::setup(settings.internal_buffer_size)?;
        let renderer_shared = Arc::new(RendererShared::new(sample_rate));
        let (resources, resource_controllers) =
            create_resources(settings.capacities, settings.main_track_builder, settings.internal_buffer_size);
        let renderer = Renderer::new(renderer_shared.clone(), settings.internal_buffer_size, resources);
        backend.start(renderer)?;
        Ok(Self {
            _backend: backend,
            resource_controllers,
            renderer_shared,
            internal_buffer_size: settings.internal_buffer_size,
        })
    }

    /// Creates a mixer sub-track.
    pub(crate) fn add_sub_track(&mut self, builder: TrackBuilder) -> Result<TrackHandle, ResourceLimitReached> {
        let (track, handle) = builder.build(self.renderer_shared.clone(), self.internal_buffer_size);
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
    /// Determines how often modulators will be updated (in samples).
    pub(crate) internal_buffer_size: usize,
}

impl Default for AudioManagerSettings {
    fn default() -> Self {
        Self {
            capacities: Capacities::default(),
            main_track_builder: MainTrackBuilder::default(),
            internal_buffer_size: 256,
        }
    }
}
