use std::collections::HashMap;
use std::ops::Mul;
use std::sync::Arc;

use cgmath::{Array, Vector2};
use derive_new::new;
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize, Timer};
use korangar_interface::elements::PrototypeElement;
use korangar_util::FileLoader;
use ragnarok_bytes::{ByteStream, FromBytes};
use ragnarok_formats::action::{Action, ActionsData};
use ragnarok_formats::version::InternalVersion;
use ragnarok_packets::ClientTick;

use super::error::LoadError;
use super::Sprite;
use crate::graphics::{Color, Texture};
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::loaders::{GameFileLoader, FALLBACK_ACTIONS_FILE};
use crate::renderer::SpriteRenderer;

#[derive(Clone, Debug, new)]
pub struct AnimationState {
    #[new(default)]
    pub action: usize,
    pub start_time: ClientTick,
    #[new(default)]
    pub time: u32,
    #[new(default)]
    pub duration: Option<u32>,
    #[new(default)]
    pub factor: Option<f32>,
}

impl AnimationState {
    pub fn idle(&mut self, client_tick: ClientTick) {
        self.action = 0;
        self.start_time = client_tick;
        self.duration = None;
        self.factor = None;
    }

    pub fn walk(&mut self, movement_speed: usize, client_tick: ClientTick) {
        self.action = 1;
        self.start_time = client_tick;
        self.duration = None;
        self.factor = Some(movement_speed as f32 * 100.0 / 150.0);
    }

    pub fn dead(&mut self, client_tick: ClientTick) {
        self.action = 8;
        self.start_time = client_tick;
        self.duration = None;
        self.factor = None;
    }

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
    #[cfg(feature = "debug")]
    actions_data: ActionsData,
}

impl Actions {
    pub fn render(
        &self,
        sprite: &Sprite,
        animation_state: &AnimationState,
        camera_direction: usize,
        head_direction: usize,
    ) -> (Arc<Texture>, Vector2<f32>, bool) {
        let direction = (camera_direction + head_direction) % 8;
        let aa = animation_state.action * 8 + direction;
        let a = &self.actions[aa % self.actions.len()];
        let delay = self.delays[aa % self.delays.len()];

        let factor = animation_state
            .factor
            .map(|factor| delay * (factor / 5.0))
            .unwrap_or_else(|| delay * 50.0);

        let frame = animation_state
            .duration
            .map(|duration| animation_state.time * a.motions.len() as u32 / duration)
            .unwrap_or_else(|| (animation_state.time as f32 / factor) as u32);
        // TODO: work out how to avoid losing digits when casting timg to an f32. When
        // fixed remove set_start_time in MouseCursor.

        let fs = &a.motions[frame as usize % a.motions.len()];

        let texture = sprite.textures[fs.sprite_clips[0].sprite_number as usize].clone();
        let texture_size = texture.get_size();
        let offset = fs.sprite_clips[0].position.map(|component| component as f32);

        (
            texture,
            Vector2::new(-offset.x, offset.y + (texture_size.height as f32) / 2.0) / 10.0,
            fs.sprite_clips[0].mirror_on != 0,
        )
    }

    pub fn render2(
        &self,
        renderer: &impl SpriteRenderer,
        sprite: &Sprite,
        animation_state: &AnimationState,
        position: ScreenPosition,
        camera_direction: usize,
        color: Color,
        application: &InterfaceSettings,
    ) {
        let direction = camera_direction % 8;
        let aa = animation_state.action * 8 + direction;
        let a = &self.actions[aa % self.actions.len()];
        let delay = self.delays[aa % self.delays.len()];

        let factor = animation_state
            .factor
            .map(|factor| delay * (factor / 5.0))
            .unwrap_or_else(|| delay * 50.0);

        let frame = animation_state
            .duration
            .map(|duration| animation_state.time * a.motions.len() as u32 / duration)
            .unwrap_or_else(|| (animation_state.time as f32 / factor) as u32);
        // TODO: work out how to avoid losing digits when casting timg to an f32. When
        // fixed remove set_start_time in MouseCursor.

        let fs = &a.motions[frame as usize % a.motions.len()];

        for sprite_clip in &fs.sprite_clips {
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
pub struct ActionLoader {
    game_file_loader: Arc<GameFileLoader>,
    cache: HashMap<String, Arc<Actions>>,
}

impl ActionLoader {
    pub fn new(game_file_loader: Arc<GameFileLoader>) -> Self {
        Self {
            game_file_loader,
            cache: HashMap::new(),
        }
    }

    fn load(&mut self, path: &str) -> Result<Arc<Actions>, LoadError> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load actions from {}", path.magenta()));

        let bytes = self
            .game_file_loader
            .get(&format!("data\\sprite\\{path}"))
            .map_err(LoadError::File)?;
        let mut byte_stream: ByteStream<Option<InternalVersion>> = ByteStream::without_metadata(&bytes);

        let actions_data = match ActionsData::from_bytes(&mut byte_stream) {
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

        #[cfg(feature = "debug")]
        let saved_actions_data = actions_data.clone();

        let delays = actions_data
            .delays
            .unwrap_or_else(|| actions_data.actions.iter().map(|_| 0.0).collect());

        let sprite = Arc::new(Actions {
            actions: actions_data.actions,
            delays,
            #[cfg(feature = "debug")]
            actions_data: saved_actions_data,
        });

        self.cache.insert(path.to_string(), sprite.clone());

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
