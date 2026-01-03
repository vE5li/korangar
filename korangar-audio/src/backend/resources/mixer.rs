use super::{ResourceController, ResourceStorage};
use crate::frame::Frame;
use crate::listener::Listener;
use crate::track::{MainTrack, MainTrackBuilder, MainTrackHandle, Track};

pub(crate) struct Mixer {
    main_track: MainTrack,
    sub_tracks: ResourceStorage<Track>,
    temp_buffer: Vec<Frame>,
}

impl Mixer {
    #[must_use]
    pub(crate) fn new(
        sub_track_capacity: usize,
        internal_buffer_size: usize,
        main_track_builder: MainTrackBuilder,
    ) -> (Self, ResourceController<Track>, MainTrackHandle) {
        let (main_track, main_track_handle) = main_track_builder.build(internal_buffer_size);
        let (sub_tracks, sub_track_controller) = ResourceStorage::new(sub_track_capacity);
        (
            Self {
                main_track,
                sub_tracks,
                temp_buffer: vec![Frame::ZERO; internal_buffer_size],
            },
            sub_track_controller,
            main_track_handle,
        )
    }

    pub(crate) fn on_start_processing(&mut self) {
        self.sub_tracks.remove_and_add(|track| track.should_be_removed());
        for track in &mut self.sub_tracks {
            track.on_start_processing();
        }
        self.main_track.on_start_processing();
    }

    pub(crate) fn process(&mut self, out: &mut [Frame], dt: f64, listener: &Listener) {
        for track in &mut self.sub_tracks {
            track.process(&mut self.temp_buffer[..out.len()], dt, listener, None);
            for (summed_out, sound_out) in out.iter_mut().zip(self.temp_buffer.iter().copied()) {
                *summed_out += sound_out;
            }
            self.temp_buffer.fill(Frame::ZERO);
        }
        self.main_track.process(out, dt);
    }
}
