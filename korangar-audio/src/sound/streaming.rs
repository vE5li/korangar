//! Decodes data gradually from an audio file.
//!
//! To play a streaming sound, pass a [`StreamingSoundData`] to
//! [`AudioManager::play`](crate::AudioManager::play).
//!
//! Streaming sounds use less memory than static sounds, but they use more
//! CPU, and they can have delays when starting or seeking.

mod data;
mod decoder;
mod handle;
mod settings;
mod sound;

use std::time::Duration;

pub(crate) use data::*;
pub(crate) use decoder::*;
pub(crate) use handle::*;
pub(crate) use settings::*;

use crate::command::{CommandReader, CommandWriter, command_writer_and_reader};

pub(crate) struct CommandWriters {
    pub(crate) stop: CommandWriter<Duration>,
}

pub(crate) struct CommandReaders {
    stop: CommandReader<Duration>,
}

#[must_use]
fn command_writers_and_readers() -> (CommandWriters, CommandReaders) {
    let (stop_writer, stop_reader) = command_writer_and_reader();
    (CommandWriters { stop: stop_writer }, CommandReaders { stop: stop_reader })
}
