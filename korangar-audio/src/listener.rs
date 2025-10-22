//! Types related to spatial listeners.
//!
//! To set the listener, use
//! [`AudioManager::set_listener`](crate::AudioManager::set_listener).
//!
//! For more information, see the documentation on [spatial mixer
//! tracks](crate::track#spatial-tracks).

use std::time::Duration;

use cgmath::{EuclideanSpace, Point3, Quaternion};

use crate::command::{CommandReader, CommandWriter, ValueChangeCommand, command_writer_and_reader};
use crate::parameter::Parameter;
use crate::tween::Tweenable;

pub(crate) struct Listener {
    pub(crate) position: Parameter<Point3<f32>>,
    pub(crate) orientation: Parameter<Quaternion<f32>>,
    pub(crate) command_readers: CommandReaders,
}

impl Listener {
    pub(crate) fn new(position: Point3<f32>, orientation: Quaternion<f32>) -> (Self, ListenerHandle) {
        let (command_writers, command_readers) = command_writers_and_readers();
        (
            Self {
                position: Parameter::new(position),
                orientation: Parameter::new(orientation),
                command_readers,
            },
            ListenerHandle { command_writers },
        )
    }

    pub(crate) fn on_start_processing(&mut self) {
        self.position.read_command(&mut self.command_readers.set_position);
        self.orientation.read_command(&mut self.command_readers.set_orientation)
    }

    pub(crate) fn update(&mut self, dt: f64) {
        self.position.update(dt);
        self.orientation.update(dt);
    }

    /// Creates a snapshot of the listener's current and previous state for
    /// spatial audio.
    #[must_use]
    pub(crate) fn listener_info(&self) -> ListenerInfo {
        ListenerInfo {
            position: self.position.value(),
            orientation: self.orientation.value(),
            previous_position: self.position.previous_value(),
            previous_orientation: self.orientation.previous_value(),
        }
    }
}

impl Default for Listener {
    fn default() -> Self {
        let (_, command_readers) = command_writers_and_readers();
        Self {
            position: Parameter::new(Point3::origin()),
            orientation: Parameter::new(Quaternion::new(1.0, 0.0, 0.0, 0.0)),
            command_readers,
        }
    }
}

/// Controls a listener.
pub(crate) struct ListenerHandle {
    pub(crate) command_writers: CommandWriters,
}

impl ListenerHandle {
    /// Sets the location of the listener in the spatial scene.
    pub(crate) fn set_position(&self, position: Point3<f32>, tween_duration: Duration) {
        self.command_writers.set_position.write(ValueChangeCommand {
            target: position,
            tween_duration,
        })
    }

    /// Sets the rotation of the listener.
    ///
    /// An unrotated listener should face in the positive Z direction with
    /// positive X to the right and positive Y up.
    pub(crate) fn set_orientation(&self, orientation: Quaternion<f32>, tween_duration: Duration) {
        self.command_writers.set_orientation.write(ValueChangeCommand {
            target: orientation,
            tween_duration,
        })
    }
}

pub(crate) struct CommandWriters {
    set_position: CommandWriter<ValueChangeCommand<Point3<f32>>>,
    set_orientation: CommandWriter<ValueChangeCommand<Quaternion<f32>>>,
}

pub(crate) struct CommandReaders {
    set_position: CommandReader<ValueChangeCommand<Point3<f32>>>,
    set_orientation: CommandReader<ValueChangeCommand<Quaternion<f32>>>,
}

#[must_use]
pub(crate) fn command_writers_and_readers() -> (CommandWriters, CommandReaders) {
    let (set_position_writer, set_position_reader) = command_writer_and_reader();
    let (set_orientation_writer, set_orientation_reader) = command_writer_and_reader();
    let command_writers = CommandWriters {
        set_position: set_position_writer,
        set_orientation: set_orientation_writer,
    };
    let command_readers = CommandReaders {
        set_position: set_position_reader,
        set_orientation: set_orientation_reader,
    };
    (command_writers, command_readers)
}

/// Information about a listener's position and orientation.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct ListenerInfo {
    /// The position of the listener.
    pub(crate) position: Point3<f32>,
    /// The rotation of the listener.
    pub(crate) orientation: Quaternion<f32>,
    /// The position of the listener prior to the last update.
    pub(crate) previous_position: Point3<f32>,
    /// The rotation of the listener prior to the last update.
    pub(crate) previous_orientation: Quaternion<f32>,
}

impl ListenerInfo {
    pub(crate) fn interpolated_position(self, amount: f32) -> Point3<f32> {
        Point3::interpolate(self.previous_position, self.position, amount as f64)
    }

    pub(crate) fn interpolated_orientation(self, amount: f32) -> Quaternion<f32> {
        Quaternion::interpolate(self.previous_orientation, self.orientation, amount as f64)
    }
}
