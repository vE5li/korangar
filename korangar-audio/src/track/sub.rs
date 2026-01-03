mod builder;
mod handle;
mod spatial_builder;
mod spatial_handle;

use std::f32::consts::FRAC_PI_8;
use std::ops::Neg;
use std::sync::Arc;

pub(crate) use builder::*;
use cgmath::{InnerSpace, Point3, Quaternion, Rad, Rotation, Rotation3, Vector3};
pub(crate) use handle::*;
pub(crate) use spatial_builder::*;
pub(crate) use spatial_handle::*;

use super::TrackShared;
use crate::backend::resources::ResourceStorage;
use crate::command::{CommandReader, CommandWriter, ValueChangeCommand, command_writer_and_reader};
use crate::decibels::Decibels;
use crate::frame::Frame;
use crate::listener::Listener;
use crate::parameter::Parameter;
use crate::playback_state_manager::PlaybackStateManager;
use crate::sound::Sound;
use crate::tween::Tweenable;

pub(crate) struct Track {
    shared: Arc<TrackShared>,
    command_readers: CommandReaders,
    volume: Parameter<Decibels>,
    sounds: ResourceStorage<Box<dyn Sound>>,
    sub_tracks: ResourceStorage<Track>,
    persist_until_sounds_finish: bool,
    spatial_data: Option<SpatialData>,
    playback_state_manager: PlaybackStateManager,
    temp_buffer: Vec<Frame>,
}

impl Track {
    #[must_use]
    pub(crate) fn shared(&self) -> Arc<TrackShared> {
        self.shared.clone()
    }

    pub(crate) fn should_be_removed(&self) -> bool {
        if self.sub_tracks.iter().any(|sub_track| !sub_track.should_be_removed()) {
            return false;
        }
        if self.persist_until_sounds_finish {
            self.shared().is_marked_for_removal() && self.sounds.is_empty()
        } else {
            self.shared().is_marked_for_removal()
        }
    }

    pub(crate) fn on_start_processing(&mut self) {
        self.read_commands();
        self.sounds.remove_and_add(|sound| sound.finished());
        for sound in &mut self.sounds {
            sound.on_start_processing();
        }
        self.sub_tracks.remove_and_add(|sub_track| sub_track.should_be_removed());
        for sub_track in &mut self.sub_tracks {
            sub_track.on_start_processing();
        }
    }

    pub(crate) fn process(&mut self, out: &mut [Frame], dt: f64, listener: &Listener, parent_spatial_position: Option<Point3<f32>>) {
        let spatial_position = self
            .spatial_data
            .as_ref()
            .map(|spatial_data| spatial_data.position.value())
            .or(parent_spatial_position);

        // Update volume parameters
        self.volume.update(dt * out.len() as f64);

        // Update playback state
        self.playback_state_manager.update(dt * out.len() as f64);

        if !self.playback_state_manager.playback_state().is_advancing() {
            out.fill(Frame::ZERO);
            return;
        }

        let num_frames = out.len();

        // Process sub tracks
        for sub_track in &mut self.sub_tracks {
            sub_track.process(&mut self.temp_buffer[..out.len()], dt, listener, spatial_position);
            for (summed_out, track_out) in out.iter_mut().zip(self.temp_buffer.iter().copied()) {
                *summed_out += track_out;
            }
            self.temp_buffer.fill(Frame::ZERO);
        }

        // Process sounds
        for sound in &mut self.sounds {
            sound.process(&mut self.temp_buffer[..out.len()], dt);
            for (summed_out, sound_out) in out.iter_mut().zip(self.temp_buffer.iter().copied()) {
                *summed_out += sound_out;
            }
            self.temp_buffer.fill(Frame::ZERO);
        }

        // Apply spatialization
        if let Some(spatial_data) = &mut self.spatial_data {
            spatial_data.position.update(dt * out.len() as f64);
            spatial_data.spatialization_strength.update(dt * out.len() as f64);

            let listener_info = listener.listener_info();

            for (i, frame) in out.iter_mut().enumerate() {
                let time_in_chunk = i as f64 / num_frames as f64;
                let interpolated_position = listener_info.interpolated_position(time_in_chunk as f32);
                let interpolated_orientation = listener_info.interpolated_orientation(time_in_chunk as f32);
                *frame = spatial_data.spatialize(*frame, interpolated_position, interpolated_orientation, time_in_chunk);
            }
        }

        // Apply volume fade
        for (i, frame) in out.iter_mut().enumerate() {
            let time_in_chunk = (i + 1) as f64 / num_frames as f64;
            let volume = self.volume.interpolated_value(time_in_chunk).as_amplitude();
            let fade_volume = self.playback_state_manager.interpolated_fade_volume(time_in_chunk).as_amplitude();
            *frame *= volume * fade_volume;
        }
    }

    fn read_commands(&mut self) {
        self.volume.read_command(&mut self.command_readers.set_volume);
    }
}

struct SpatialData {
    position: Parameter<Point3<f32>>,
    /// The distances from a listener at which the track is loudest and
    /// quietest.
    distances: SpatialTrackDistances,
    /// How the track's volume will change with distance.
    ///
    /// If `false`, the track will output at a constant volume.
    use_linear_attenuation_function: bool,
    /// How much the track's output should be panned left or right depending on
    /// its direction from the listener.
    ///
    /// This value should be between `0.0` and `1.0`. `0.0` disables
    /// spatialization entirely.
    spatialization_strength: Parameter<f32>,
}

impl SpatialData {
    fn spatialize(&self, input: Frame, listener_position: Point3<f32>, listener_orientation: Quaternion<f32>, time_in_chunk: f64) -> Frame {
        let position = self.position.interpolated_value(time_in_chunk);
        let spatialization_strength = self.spatialization_strength.interpolated_value(time_in_chunk).clamp(0.0, 1.0);
        let min_ear_amplitude = 1.0 - spatialization_strength;

        let mut output = input;

        if self.use_linear_attenuation_function {
            // Attenuate volume
            let distance = (listener_position - position).magnitude();
            let relative_distance = self.distances.relative_distance(distance);
            let relative_volume = 1.0 - relative_distance;
            let amplitude = Tweenable::interpolate(Decibels::SILENCE, Decibels::IDENTITY, relative_volume.into()).as_amplitude();
            output *= amplitude;
        }

        if spatialization_strength != 0.0 {
            // Apply spatialization
            output = output.as_mono();
            let (left_ear_position, right_ear_position) = listener_ear_positions(listener_position, listener_orientation);
            let (left_ear_direction, right_ear_direction) = listener_ear_directions(listener_orientation);
            let emitter_direction_relative_to_left_ear = (position - left_ear_position).normalize();
            let emitter_direction_relative_to_right_ear = (position - right_ear_position).normalize();
            let left_ear_volume = (left_ear_direction.dot(emitter_direction_relative_to_left_ear) + 1.0) / 2.0;
            let right_ear_volume = (right_ear_direction.dot(emitter_direction_relative_to_right_ear) + 1.0) / 2.0;
            output.left *= min_ear_amplitude + (1.0 - min_ear_amplitude) * left_ear_volume;
            output.right *= min_ear_amplitude + (1.0 - min_ear_amplitude) * right_ear_volume;
        }

        output
    }
}

#[must_use]
fn listener_ear_positions(listener_position: Point3<f32>, listener_orientation: Quaternion<f32>) -> (Point3<f32>, Point3<f32>) {
    const EAR_DISTANCE: f32 = 0.1;
    let position = listener_position;
    let orientation = listener_orientation;
    let left = position + orientation.rotate_vector(Vector3::unit_x().neg() * EAR_DISTANCE);
    let right = position + orientation.rotate_vector(Vector3::unit_x() * EAR_DISTANCE);
    (left, right)
}

#[must_use]
fn listener_ear_directions(listener_orientation: Quaternion<f32>) -> (Vector3<f32>, Vector3<f32>) {
    const EAR_ANGLE_FROM_HEAD: f32 = FRAC_PI_8;
    let left_ear_direction_relative_to_head = Quaternion::from_angle_y(Rad(-EAR_ANGLE_FROM_HEAD)) * Vector3::unit_x().neg();
    let right_ear_direction_relative_to_head = Quaternion::from_angle_y(Rad(EAR_ANGLE_FROM_HEAD)) * Vector3::unit_x();
    let orientation = listener_orientation;
    let left = orientation * left_ear_direction_relative_to_head;
    let right = orientation * right_ear_direction_relative_to_head;
    (left, right)
}

pub(crate) struct CommandWriters {
    set_volume: CommandWriter<ValueChangeCommand<Decibels>>,
}

pub(crate) struct CommandReaders {
    set_volume: CommandReader<ValueChangeCommand<Decibels>>,
}

#[must_use]
pub(crate) fn command_writers_and_readers() -> (CommandWriters, CommandReaders) {
    let (set_volume_writer, set_volume_reader) = command_writer_and_reader();
    let command_writers = CommandWriters {
        set_volume: set_volume_writer,
    };
    let command_readers = CommandReaders {
        set_volume: set_volume_reader,
    };
    (command_writers, command_readers)
}
