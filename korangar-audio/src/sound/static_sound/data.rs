use std::io::Cursor;
use std::sync::Arc;

use symphonia::core::io::{MediaSource, MediaSourceStream};

use super::StaticSoundSettings;
use super::handle::StaticSoundHandle;
use super::sound::StaticSound;
use crate::decibels::Decibels;
use crate::frame::Frame;
use crate::sound::error::FromFileError;
use crate::sound::symphonia::load_frames_from_buffer_ref;
use crate::sound::{Sound, SoundData};

/// A piece of audio loaded into memory all at once.
///
/// These can be cheaply cloned, as the audio data is shared
/// among all clones.
#[derive(Clone, PartialEq)]
pub(crate) struct StaticSoundData {
    /// The sample rate of the audio (in Hz).
    pub(crate) sample_rate: u32,
    /// The raw samples that make up the audio.
    pub(crate) frames: Arc<[Frame]>,
    /// Settings for the sound.
    pub(crate) settings: StaticSoundSettings,
}

impl StaticSoundData {
    /// Sets the volume of the sound.
    #[must_use]
    pub(crate) fn volume(&self, volume: Decibels) -> Self {
        let mut new = self.clone();
        new.settings.volume = volume;
        new
    }

    /// Returns the number of frames in the [`StaticSoundData`].
    #[must_use]
    pub(crate) fn num_frames(&self) -> usize {
        self.frames.len()
    }

    /// Loads a cursor wrapping audio file data into a [`StaticSoundData`].
    pub(crate) fn from_cursor<T: AsRef<[u8]> + Send + Sync + 'static>(cursor: Cursor<T>) -> Result<StaticSoundData, FromFileError> {
        Self::from_media_source(cursor)
    }

    /// Loads an audio file from a type that implements Symphonia's
    /// [`MediaSource`] trait.
    pub(crate) fn from_media_source(media_source: impl MediaSource + 'static) -> Result<Self, FromFileError> {
        Self::from_boxed_media_source(Box::new(media_source))
    }

    fn from_boxed_media_source(media_source: Box<dyn MediaSource>) -> Result<Self, FromFileError> {
        let codecs = symphonia::default::get_codecs();
        let probe = symphonia::default::get_probe();
        let media_source_stream = MediaSourceStream::new(media_source, Default::default());
        let mut format_reader = probe
            .format(
                &Default::default(),
                media_source_stream,
                &Default::default(),
                &Default::default(),
            )?
            .format;
        let default_track = format_reader.default_track().ok_or(FromFileError::NoDefaultTrack)?;
        let default_track_id = default_track.id;
        let codec_params = &default_track.codec_params;
        let sample_rate = codec_params.sample_rate.ok_or(FromFileError::UnknownSampleRate)?;
        let mut decoder = codecs.make(codec_params, &Default::default())?;
        let mut frames = vec![];
        loop {
            match format_reader.next_packet() {
                Ok(packet) => {
                    if default_track_id == packet.track_id() {
                        let buffer = decoder.decode(&packet)?;
                        frames.append(&mut load_frames_from_buffer_ref(&buffer)?);
                    }
                }
                Err(error) => {
                    return match error {
                        symphonia::core::errors::Error::IoError(error) => {
                            if error.kind() == std::io::ErrorKind::UnexpectedEof {
                                break;
                            }
                            Err(symphonia::core::errors::Error::IoError(error).into())
                        }
                        error => Err(error.into()),
                    };
                }
            }
        }
        Ok(Self {
            sample_rate,
            frames: frames.into(),
            settings: StaticSoundSettings::default(),
        })
    }

    pub(super) fn split(self) -> (StaticSound, StaticSoundHandle) {
        let sound = StaticSound::new(self);
        let shared = sound.shared();
        (sound, StaticSoundHandle { shared })
    }
}

impl SoundData for StaticSoundData {
    type Error = ();
    type Handle = StaticSoundHandle;

    #[allow(clippy::type_complexity)]
    fn into_sound(self) -> Result<(Box<dyn Sound>, Self::Handle), Self::Error> {
        let (sound, handle) = self.split();
        Ok((Box::new(sound), handle))
    }
}

pub(crate) fn num_frames(frames: &[Frame]) -> usize {
    frames.len()
}

pub(crate) fn frame_at_index(index: usize, frames: &[Frame]) -> Option<Frame> {
    frames.get(index).copied()
}
