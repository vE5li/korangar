use std::time::Duration;

use crate::backend::resources::ResourceStorage;
use crate::command::{CommandReader, CommandWriter, ValueChangeCommand, command_writer_and_reader};
use crate::decibels::Decibels;
use crate::frame::Frame;
use crate::parameter::Parameter;
use crate::sound::Sound;

pub(crate) struct MainTrack {
    volume: Parameter<Decibels>,
    set_volume_command_reader: CommandReader<ValueChangeCommand<Decibels>>,
    sounds: ResourceStorage<Box<dyn Sound>>,
    temp_buffer: Vec<Frame>,
}

impl MainTrack {
    pub(crate) fn on_start_processing(&mut self) {
        self.volume.read_command(&mut self.set_volume_command_reader);
        self.sounds.remove_and_add(|sound| sound.finished());
        for sound in &mut self.sounds {
            sound.on_start_processing();
        }
    }

    pub(crate) fn process(&mut self, out: &mut [Frame], dt: f64) {
        self.volume.update(dt * out.len() as f64);
        for sound in &mut self.sounds {
            sound.process(&mut self.temp_buffer[..out.len()], dt);
            for (summed_out, sound_out) in out.iter_mut().zip(self.temp_buffer.iter().copied()) {
                *summed_out += sound_out;
            }
            self.temp_buffer.fill(Frame::ZERO);
        }
        let num_frames = out.len();
        for (i, frame) in out.iter_mut().enumerate() {
            let time_in_chunk = (i + 1) as f64 / num_frames as f64;
            let volume = self.volume.interpolated_value(time_in_chunk).as_amplitude();
            *frame *= volume;
        }
    }
}

/// Configures the main mixer track.
pub(crate) struct MainTrackBuilder {
    /// The volume of the track.
    pub(crate) volume: Decibels,
    /// The maximum number of sounds that can be played simultaneously on this
    /// track.
    pub(crate) sound_capacity: usize,
}

impl MainTrackBuilder {
    /// Creates a new [`MainTrackBuilder`] with the default settings.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self {
            volume: Decibels::IDENTITY,
            sound_capacity: 128,
        }
    }

    #[must_use]
    pub(crate) fn build(self, internal_buffer_size: usize) -> (MainTrack, MainTrackHandle) {
        let (set_volume_command_writer, set_volume_command_reader) = command_writer_and_reader();
        let (sounds, _sound_controller) = ResourceStorage::new(self.sound_capacity);
        let track = MainTrack {
            volume: Parameter::new(self.volume),
            set_volume_command_reader,
            sounds,
            temp_buffer: vec![Frame::ZERO; internal_buffer_size],
        };
        let handle = MainTrackHandle { set_volume_command_writer };
        (track, handle)
    }
}

impl Default for MainTrackBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Controls the main mixer track.
pub(crate) struct MainTrackHandle {
    pub(crate) set_volume_command_writer: CommandWriter<ValueChangeCommand<Decibels>>,
}

impl MainTrackHandle {
    /// Sets the (post-effects) volume of the mixer track.
    pub(crate) fn set_volume(&mut self, volume: Decibels, tween_duration: Duration) {
        self.set_volume_command_writer.write(ValueChangeCommand {
            target: volume,
            tween_duration,
        })
    }
}
