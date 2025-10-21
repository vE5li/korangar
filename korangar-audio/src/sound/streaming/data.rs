use std::sync::Arc;

use super::{StreamingSoundHandle, StreamingSoundSettings, SymphoniaDecoder, command_writers_and_readers};
use crate::sound::error::FromFileError;
use crate::sound::streaming::sound::decode_scheduler::DecodeScheduler;
use crate::sound::streaming::sound::{Shared, StreamingSound};
use crate::sound::{Sound, SoundData};

/// A streaming sound that is not playing yet.
pub(crate) struct StreamingSoundData {
    pub(crate) decoder: SymphoniaDecoder,
    /// Settings for the streaming sound.
    pub(crate) settings: StreamingSoundSettings,
}

impl StreamingSoundData {
    /// Creates a [`StreamingSoundData`] for an audio file.
    pub(crate) fn from_file(path: impl AsRef<std::path::Path>, loops: bool) -> Result<StreamingSoundData, FromFileError> {
        Ok(Self {
            decoder: SymphoniaDecoder::new(Box::new(std::fs::File::open(path)?))?,
            settings: StreamingSoundSettings {
                loops,
                ..Default::default()
            },
        })
    }
}

impl StreamingSoundData {
    pub(crate) fn split(self, backend_sample_rate: u32) -> Result<(StreamingSound, StreamingSoundHandle, DecodeScheduler), FromFileError> {
        let (command_writers, command_readers) = command_writers_and_readers();
        let sample_rate = self.decoder.sample_rate();
        let shared = Arc::new(Shared::new());
        let (scheduler, frame_consumer) = DecodeScheduler::new(self.decoder, self.settings, shared.clone(), backend_sample_rate)?;
        let sound = StreamingSound::new(
            sample_rate,
            self.settings,
            shared.clone(),
            frame_consumer,
            command_readers,
            &scheduler,
        );
        let handle = StreamingSoundHandle { shared, command_writers };
        Ok((sound, handle, scheduler))
    }
}

impl SoundData for StreamingSoundData {
    type Error = FromFileError;
    type Handle = StreamingSoundHandle;

    fn into_sound(self, backend_sample_rate: u32) -> Result<(Box<dyn Sound>, Self::Handle), Self::Error> {
        let (sound, handle, scheduler) = self.split(backend_sample_rate)?;
        scheduler.start();
        Ok((Box::new(sound), handle))
    }
}
