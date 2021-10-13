use graphics::RenderSettings;

use super::{ StateKey, StateValue };

pub struct StateProvider<'t> {
    render_settings: &'t RenderSettings,
}

impl<'t> StateProvider<'t>{

    pub fn new(render_settings: &'t RenderSettings) -> Self {
        return Self { render_settings };
    }

    pub fn get(&self, key: &StateKey) -> StateValue {
        match key {
            StateKey::ShowFramesPerSecond => StateValue::Boolean(self.render_settings.show_frames_per_second),
            StateKey::ShowMap => StateValue::Boolean(self.render_settings.show_map),
            StateKey::ShowObjects => StateValue::Boolean(self.render_settings.show_objects),
            StateKey::ShowAmbientLight => StateValue::Boolean(self.render_settings.show_ambient_light),
            StateKey::ShowDirectionalLight => StateValue::Boolean(self.render_settings.show_directional_light),
            StateKey::ShowPointLights => StateValue::Boolean(self.render_settings.show_point_lights),
            StateKey::ShowParticleLights => StateValue::Boolean(self.render_settings.show_particle_lights),
            #[cfg(feature = "debug")]
            StateKey::UseDebugCamera => StateValue::Boolean(self.render_settings.use_debug_camera),
            #[cfg(feature = "debug")]
            StateKey::ShowObjectMarkers => StateValue::Boolean(self.render_settings.show_object_markers),
            #[cfg(feature = "debug")]
            StateKey::ShowLightMarkers => StateValue::Boolean(self.render_settings.show_light_markers),
            #[cfg(feature = "debug")]
            StateKey::ShowSoundMarkers => StateValue::Boolean(self.render_settings.show_sound_markers),
            #[cfg(feature = "debug")]
            StateKey::ShowEffectMarkers => StateValue::Boolean(self.render_settings.show_effect_markers),
            #[cfg(feature = "debug")]
            StateKey::ShowParticleMarkers => StateValue::Boolean(self.render_settings.show_particle_markers),
            #[cfg(feature = "debug")]
            StateKey::ShowMapTiles => StateValue::Boolean(self.render_settings.show_map_tiles),
        }
    }
}
