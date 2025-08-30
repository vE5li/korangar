use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::{Arc, Mutex};

use korangar_audio::AudioEngine;
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_util::FileLoader;
use korangar_util::container::SimpleCache;
use ragnarok_bytes::{ByteReader, FromBytes};
use ragnarok_formats::action::ActionsData;
use ragnarok_formats::version::InternalVersion;

use super::error::LoadError;
use crate::loaders::{FALLBACK_ACTIONS_FILE, GameFileLoader};
use crate::world::{ActionEvent, Actions};

const MAX_CACHE_COUNT: u32 = 256;
// We cache actions only by count.
const MAX_CACHE_SIZE: usize = usize::MAX;

pub struct ActionLoader {
    game_file_loader: Arc<GameFileLoader>,
    audio_engine: Arc<AudioEngine<GameFileLoader>>,
    cache: Mutex<SimpleCache<String, Arc<Actions>>>,
}

impl ActionLoader {
    pub fn new(game_file_loader: Arc<GameFileLoader>, audio_engine: Arc<AudioEngine<GameFileLoader>>) -> Self {
        Self {
            game_file_loader,
            audio_engine,
            cache: Mutex::new(SimpleCache::new(
                NonZeroU32::new(MAX_CACHE_COUNT).unwrap(),
                NonZeroUsize::new(MAX_CACHE_SIZE).unwrap(),
            )),
        }
    }

    fn load(&self, path: &str) -> Result<Arc<Actions>, LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load actions from {}", path.magenta()));

        let bytes = match self.game_file_loader.get(&format!("data\\sprite\\{path}")) {
            Ok(bytes) => bytes,
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to load actions: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                return self.get_or_load(FALLBACK_ACTIONS_FILE);
            }
        };
        let mut byte_reader: ByteReader<Option<InternalVersion>> = ByteReader::with_default_metadata(&bytes);

        let actions_data = match ActionsData::from_bytes(&mut byte_reader) {
            Ok(actions_data) => actions_data,
            Err(_error) => {
                #[cfg(feature = "debug")]
                {
                    print_debug!("Failed to load actions: {:?}", _error);
                    print_debug!("Replacing with fallback");
                }

                return self.get_or_load(FALLBACK_ACTIONS_FILE);
            }
        };

        #[allow(clippy::unused_enumerate_index)]
        let events: Vec<ActionEvent> = actions_data
            .events
            .iter()
            .enumerate()
            .map(|(_index, event)| {
                if event.name.ends_with(".wav") {
                    let key = self.audio_engine.load(&event.name);
                    ActionEvent::Sound { key }
                } else if event.name == "atk" || event.name == "atk.txt" {
                    ActionEvent::Attack
                } else {
                    #[cfg(feature = "debug")]
                    print_debug!("Found unknown event at index `{}`: {:?}", _index, event.name);
                    ActionEvent::Unknown
                }
            })
            .collect();

        #[cfg(feature = "debug")]
        let saved_actions_data = actions_data.clone();

        let delays = actions_data
            .delays
            .unwrap_or_else(|| actions_data.actions.iter().map(|_| 0.0).collect());

        let sprite = Arc::new(Actions {
            actions: actions_data.actions,
            delays,
            events,
            #[cfg(feature = "debug")]
            actions_data: saved_actions_data,
        });

        let _result = self.cache.lock().unwrap().insert(path.to_string(), sprite.clone());

        #[cfg(feature = "debug")]
        if let Err(error) = _result {
            print_debug!(
                "[{}] action could not be added to cache. Path: '{}': {:?}",
                "error".red(),
                &path,
                error
            );
        }

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(sprite)
    }

    pub fn get_or_load(&self, path: &str) -> Result<Arc<Actions>, LoadError> {
        let Some(action) = self.cache.lock().unwrap().get(path).cloned() else {
            return self.load(path);
        };

        Ok(action)
    }
}
