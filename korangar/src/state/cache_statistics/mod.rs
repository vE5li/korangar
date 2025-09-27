use korangar_audio::AudioEngine;
use korangar_interface::element::StateElement;
use korangar_interface::window::StateWindow;
use rust_state::RustState;

use crate::interface::windows::WindowClass;
use crate::loaders::{ActionLoader, AnimationLoader, EffectLoader, FontLoader, GameFileLoader, SpriteLoader, TextureLoader};

#[derive(Clone, Copy, PartialEq, Default, RustState, StateElement, StateWindow)]
#[window_class(WindowClass::CacheStatistics)]
#[window_title("Cache Statistics")]
pub struct CacheStatistics {
    texture_cache: korangar_container::CacheStatistics,
    sprite_cache: korangar_container::CacheStatistics,
    font_cache: korangar_container::CacheStatistics,
    sound_cache: korangar_container::CacheStatistics,
    action_cache: korangar_container::CacheStatistics,
    animation_cache: korangar_container::CacheStatistics,
    effect_cache: korangar_container::CacheStatistics,
    #[hidden_element]
    last_update: f64,
}

impl CacheStatistics {
    pub fn update(
        &mut self,
        delta_time: f64,
        texture_loader: &TextureLoader,
        sprite_loader: &SpriteLoader,
        font_loader: &FontLoader,
        audio_engine: &AudioEngine<GameFileLoader>,
        action_loader: &ActionLoader,
        animation_loader: &AnimationLoader,
        effect_loader: &EffectLoader,
    ) {
        self.last_update += delta_time;

        if self.last_update >= 1.0 {
            self.last_update = 0.0;

            self.texture_cache = texture_loader.cache_statistics();
            self.sprite_cache = sprite_loader.cache_statistics();
            self.font_cache = font_loader.cache_statistics();
            self.sound_cache = audio_engine.cache_statistics();
            self.action_cache = action_loader.cache_statistics();
            self.animation_cache = animation_loader.cache_statistics();
            self.effect_cache = effect_loader.cache_statistics();
        }
    }
}
