//! Playable chunks of audio that are loaded into memory all at once.
//!
//! To play a static sound, pass a [`StaticSoundData`] to
//! [`AudioManager::play`](crate::AudioManager::play).
//!
//! Compared to streaming sounds, static sounds have lower CPU usage and shorter
//! delays when starting and seeking, but they use a lot more memory.

mod data;
mod handle;
mod settings;
mod sound;

pub(crate) use data::*;
pub(crate) use handle::*;
pub(crate) use settings::*;
