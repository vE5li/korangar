use graphics::RenderSettings;
use Entity;

use super::{ StateKey, StateValue };

pub struct StateProvider<'t> {
    render_settings: &'t RenderSettings,
    player: &'t Entity,
}

impl<'t> StateProvider<'t>{

    pub fn new(render_settings: &'t RenderSettings, player: &'t Entity) -> Self {
        return Self { render_settings, player };
    }

    pub fn get(&self, key: &StateKey) -> StateValue {
        match key {
            StateKey::ShowFramesPerSecond => StateValue::Boolean(self.render_settings.show_frames_per_second),
            StateKey::ShowMap => StateValue::Boolean(self.render_settings.show_map),
            StateKey::ShowObjects => StateValue::Boolean(self.render_settings.show_objects),
            StateKey::ShowEntities => StateValue::Boolean(self.render_settings.show_entities),
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
            #[cfg(feature = "debug")]
            StateKey::ShowPathing => StateValue::Boolean(self.render_settings.show_pathing),
            #[cfg(feature = "debug")]
            StateKey::ShowDiffuseBuffer => StateValue::Boolean(self.render_settings.show_diffuse_buffer),
            #[cfg(feature = "debug")]
            StateKey::ShowNormalBuffer => StateValue::Boolean(self.render_settings.show_normal_buffer),
            #[cfg(feature = "debug")]
            StateKey::ShowDepthBuffer => StateValue::Boolean(self.render_settings.show_depth_buffer),
            StateKey::PlayerMaximumHealthPoints => StateValue::Number(self.player.maximum_health_points),
            StateKey::PlayerMaximumSpellPoints => StateValue::Number(self.player.maximum_spell_points),
            StateKey::PlayerMaximumActivityPoints => StateValue::Number(self.player.maximum_activity_points),
            StateKey::PlayerCurrentHealthPoints => StateValue::Number(self.player.current_health_points),
            StateKey::PlayerCurrentSpellPoints => StateValue::Number(self.player.current_spell_points),
            StateKey::PlayerCurrentActivityPoints => StateValue::Number(self.player.current_activity_points),
        }
    }
}
