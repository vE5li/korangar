use std::num::{NonZeroU32, NonZeroUsize};
use std::ops::Mul;
use std::sync::Arc;

use cgmath::{Array, Vector2};
use derive_new::new;
use korangar_audio::{AudioEngine, SoundEffectKey};
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize, Timer};
use korangar_interface::elements::{ElementCell, PrototypeElement};
use korangar_util::container::{Cacheable, SimpleCache};
use korangar_util::FileLoader;
use ragnarok_bytes::{ByteReader, FromBytes};
use ragnarok_formats::action::{Action, ActionsData};
use ragnarok_formats::version::InternalVersion;
use ragnarok_packets::ClientTick;

use super::error::LoadError;
use super::Sprite;
use crate::graphics::Color;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::loaders::{GameFileLoader, FALLBACK_ACTIONS_FILE};
use crate::renderer::SpriteRenderer;

const MAX_CACHE_COUNT: u32 = 256;
const MAX_CACHE_SIZE: usize = 64 * 1024 * 1024;

// TODO: NHA The numeric value of action types are based on the EntityType!
//       For example "Dead" is 8 for the PC and 4 for a monster.
//       This means we need to refactor the AnimationState, so that the mouse
//       uses a different animation state struct (since we can't do simple usize
//       conversions).
#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum ActionType {
    #[default]
    Idle = 0,
    Walk = 1,
    Dead = 8,
}

impl From<ActionType> for usize {
    fn from(value: ActionType) -> Self {
        value as usize
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ActionEvent {
    /// Start playing a WAV sound file.
    Sound { key: SoundEffectKey },
    /// An attack event when the "flinch" animation is played.
    Attack,
    /// Start playing a WAV sound file.
    Unknown,
}

impl PrototypeElement<InterfaceSettings> for ActionEvent {
    fn to_element(&self, display: String) -> ElementCell<InterfaceSettings> {
        match self {
            Self::Sound { .. } => PrototypeElement::to_element(&"Sound", display),
            Self::Attack => PrototypeElement::to_element(&"Attack", display),
            Self::Unknown => PrototypeElement::to_element(&"Unknown", display),
        }
    }
}

#[derive(Clone, Debug, new)]
pub struct AnimationState<T = ActionType> {
    pub action: T,
    pub start_time: ClientTick,
    #[new(default)]
    pub time: u32,
    #[new(default)]
    pub duration: Option<u32>,
    #[new(default)]
    pub factor: Option<f32>,
}

impl AnimationState<ActionType> {
    pub fn idle(&mut self, client_tick: ClientTick) {
        self.action = ActionType::Idle;
        self.start_time = client_tick;
        self.duration = None;
        self.factor = None;
    }

    pub fn walk(&mut self, movement_speed: usize, client_tick: ClientTick) {
        self.action = ActionType::Walk;
        self.start_time = client_tick;
        self.duration = None;
        self.factor = Some(movement_speed as f32 * 100.0 / 150.0);
    }

    pub fn dead(&mut self, client_tick: ClientTick) {
        self.action = ActionType::Dead;
        self.start_time = client_tick;
        self.duration = None;
        self.factor = None;
    }
}

impl<T> AnimationState<T> {
    pub fn update(&mut self, client_tick: ClientTick) {
        let mut time = client_tick.0.saturating_sub(self.start_time.0);

        // TODO: make everything have a duration so that we can update the start_time
        // from time to time so that animations won't start to drop frames as
        // soon as start_time - client_tick can no longer be stored in an f32
        // accurately. When fixed remove set_start_time in MouseCursor.
        if let Some(duration) = self.duration
            && time > duration
        {
            //self.action = self.next_action;
            self.start_time = client_tick;
            self.duration = None;

            time = 0;
        }

        self.time = time;
    }
}

#[derive(Debug, PrototypeElement)]
pub struct Actions {
    pub actions: Vec<Action>,
    pub delays: Vec<f32>,
    #[hidden_element]
    pub events: Vec<ActionEvent>,
    #[cfg(feature = "debug")]
    actions_data: ActionsData,
}

impl Actions {
    pub fn render<T>(
        &self,
        renderer: &impl SpriteRenderer,
        sprite: &Sprite,
        animation_state: &AnimationState<T>,
        position: ScreenPosition,
        camera_direction: usize,
        color: Color,
        application: &InterfaceSettings,
    ) where
        T: Into<usize> + Copy,
    {
        let direction = camera_direction % 8;
        let animation_action = animation_state.action.into() * 8 + direction;
        let action = &self.actions[animation_action % self.actions.len()];
        let delay = self.delays[animation_action % self.delays.len()];

        let factor = animation_state
            .factor
            .map(|factor| delay * (factor / 5.0))
            .unwrap_or_else(|| delay * 50.0);

        let frame = animation_state
            .duration
            .map(|duration| animation_state.time * action.motions.len() as u32 / duration)
            .unwrap_or_else(|| (animation_state.time as f32 / factor) as u32);
        // TODO: work out how to avoid losing digits when casting timing to an f32. When
        // fixed remove set_start_time in MouseCursor.

        let motion = &action.motions[frame as usize % action.motions.len()];

        for sprite_clip in &motion.sprite_clips {
            // `get` instead of a direct index in case a fallback was loaded
            let Some(texture) = sprite.textures.get(sprite_clip.sprite_number as usize) else {
                return;
            };

            let offset = sprite_clip.position.map(|component| component as f32);
            let dimesions = sprite_clip
                .size
                .unwrap_or_else(|| {
                    let image_size = texture.get_size();
                    Vector2::new(image_size.width, image_size.height)
                })
                .map(|component| component as f32);
            let zoom = sprite_clip.zoom.unwrap_or(1.0) * application.get_scaling_factor();
            let zoom2 = sprite_clip.zoom2.unwrap_or_else(|| Vector2::from_value(1.0));

            let final_size = dimesions.zip(zoom2, f32::mul) * zoom;
            let final_position = Vector2::new(position.left, position.top) + offset - final_size / 2.0;

            let final_size = ScreenSize {
                width: final_size.x,
                height: final_size.y,
            };

            let final_position = ScreenPosition {
                left: final_position.x,
                top: final_position.y,
            };

            let screen_clip = ScreenClip {
                left: 0.0,
                top: 0.0,
                right: f32::MAX,
                bottom: f32::MAX,
            };

            renderer.render_sprite(texture.clone(), final_position, final_size, screen_clip, color, false);
        }
    }
}

impl Cacheable for Actions {
    fn size(&self) -> usize {
        size_of_val(&self.actions)
    }
}

pub struct ActionLoader {
    game_file_loader: Arc<GameFileLoader>,
    audio_engine: Arc<AudioEngine<GameFileLoader>>,
    cache: SimpleCache<String, Arc<Actions>>,
}

impl ActionLoader {
    pub fn new(game_file_loader: Arc<GameFileLoader>, audio_engine: Arc<AudioEngine<GameFileLoader>>) -> Self {
        Self {
            game_file_loader,
            audio_engine,
            cache: SimpleCache::new(
                NonZeroU32::new(MAX_CACHE_COUNT).unwrap(),
                NonZeroUsize::new(MAX_CACHE_SIZE).unwrap(),
            ),
        }
    }

    fn load(&mut self, path: &str) -> Result<Arc<Actions>, LoadError> {
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

                return self.get(FALLBACK_ACTIONS_FILE);
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

                return self.get(FALLBACK_ACTIONS_FILE);
            }
        };

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

        self.cache.insert(path.to_string(), sprite.clone()).unwrap();

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(sprite)
    }

    pub fn get(&mut self, path: &str) -> Result<Arc<Actions>, LoadError> {
        match self.cache.get(path) {
            Some(sprite) => Ok(sprite.clone()),
            None => self.load(path),
        }
    }
}
