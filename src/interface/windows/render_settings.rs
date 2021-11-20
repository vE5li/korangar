use cgmath::Vector2;

use input::UserEvent;
use graphics::Color; // temp

use super::super::*;

pub fn render_settings_window(window_builder: &mut WindowBuilder, interface_state: &mut InterfaceState) -> Element {

    let position = Vector2::new(800.0, 20.0);

    let frames_per_second_button = checkbutton(window_builder, String::from("frame counter"), UserEvent::ToggleShowFramesPerSecond, StateKey::ShowFramesPerSecond, window_builder.inner_width());
    let map_button = checkbutton(window_builder, String::from("map"), UserEvent::ToggleShowMap, StateKey::ShowMap, window_builder.inner_width());
    let objects_button = checkbutton(window_builder, String::from("objects"), UserEvent::ToggleShowObjects, StateKey::ShowObjects, window_builder.inner_width());
    let entities_button = checkbutton(window_builder, String::from("entities"), UserEvent::ToggleShowEntities, StateKey::ShowEntities, window_builder.inner_width());

    window_builder.new_row_spaced(6.0);
    let lights_text = text(window_builder, String::from("lights"), Color::new(100, 100, 100), 14.0);
    let ambient_light_button = checkbutton(window_builder, String::from("ambient light"), UserEvent::ToggleShowAmbientLight, StateKey::ShowAmbientLight, window_builder.inner_width());
    let directional_light_button = checkbutton(window_builder, String::from("directional light"), UserEvent::ToggleShowDirectionalLight, StateKey::ShowDirectionalLight, window_builder.inner_width());
    let point_lights_button = checkbutton(window_builder, String::from("point lights"), UserEvent::ToggleShowPointLights, StateKey::ShowPointLights, window_builder.inner_width());
    let particle_lights_button = checkbutton(window_builder, String::from("particle lights"), UserEvent::ToggleShowParticleLights, StateKey::ShowParticleLights, window_builder.inner_width());

    #[cfg(feature = "debug")]
    window_builder.new_row_spaced(6.0);
    #[cfg(feature = "debug")]
    let camera_text = text(window_builder, String::from("camera"), Color::new(100, 100, 100), 14.0);
    #[cfg(feature = "debug")]
    let debug_camera_button = checkbutton(window_builder, String::from("debug camera"), UserEvent::ToggleUseDebugCamera, StateKey::UseDebugCamera, window_builder.inner_width());

    #[cfg(feature = "debug")]
    window_builder.new_row_spaced(6.0);
    #[cfg(feature = "debug")]
    let markers_text = text(window_builder, String::from("markers"), Color::new(100, 100, 100), 14.0);
    #[cfg(feature = "debug")]
    let object_markers_button = checkbutton(window_builder, String::from("object markers"), UserEvent::ToggleShowObjectMarkers, StateKey::ShowObjectMarkers, window_builder.inner_width());
    #[cfg(feature = "debug")]
    let light_markers_button = checkbutton(window_builder, String::from("light markers"), UserEvent::ToggleShowLightMarkers, StateKey::ShowLightMarkers, window_builder.inner_width());
    #[cfg(feature = "debug")]
    let sound_markers_button = checkbutton(window_builder, String::from("sound markers"), UserEvent::ToggleShowSoundMarkers, StateKey::ShowSoundMarkers, window_builder.inner_width());
    #[cfg(feature = "debug")]
    let effect_markers_button = checkbutton(window_builder, String::from("effect markers"), UserEvent::ToggleShowEffectMarkers, StateKey::ShowEffectMarkers, window_builder.inner_width());
    #[cfg(feature = "debug")]
    let particle_markers_button = checkbutton(window_builder, String::from("particle markers"), UserEvent::ToggleShowParticleMarkers, StateKey::ShowParticleMarkers, window_builder.inner_width());

    #[cfg(feature = "debug")]
    window_builder.new_row_spaced(6.0);
    #[cfg(feature = "debug")]
    let grid_text = text(window_builder, String::from("grid"), Color::new(100, 100, 100), 14.0);
    #[cfg(feature = "debug")]
    let tiles_button = checkbutton(window_builder, String::from("map tiles"), UserEvent::ToggleShowMapTiles, StateKey::ShowMapTiles, window_builder.inner_width());
    #[cfg(feature = "debug")]
    let pathing_button = checkbutton(window_builder, String::from("pathing"), UserEvent::ToggleShowPathing, StateKey::ShowPathing, window_builder.inner_width());

    #[cfg(feature = "debug")]
    window_builder.new_row_spaced(6.0);
    #[cfg(feature = "debug")]
    let buffers_text = text(window_builder, String::from("buffers"), Color::new(100, 100, 100), 14.0);
    #[cfg(feature = "debug")]
    let diffuse_buffer_button = checkbutton(window_builder, String::from("diffuse buffer"), UserEvent::ToggleShowDiffuseBuffer, StateKey::ShowDiffuseBuffer, window_builder.inner_width());
    #[cfg(feature = "debug")]
    let normal_buffer_button = checkbutton(window_builder, String::from("normal buffer"), UserEvent::ToggleShowNormalBuffer, StateKey::ShowNormalBuffer, window_builder.inner_width());
    #[cfg(feature = "debug")]
    let depth_buffer_button = checkbutton(window_builder, String::from("depth buffer"), UserEvent::ToggleShowDepthBuffer, StateKey::ShowDepthBuffer, window_builder.inner_width());

    let elements = vec![
        frames_per_second_button,
        map_button,
        objects_button,
        entities_button,
        lights_text,
        ambient_light_button,
        directional_light_button,
        point_lights_button,
        particle_lights_button,
        #[cfg(feature = "debug")]
        camera_text,
        #[cfg(feature = "debug")]
        debug_camera_button,
        #[cfg(feature = "debug")]
        markers_text,
        #[cfg(feature = "debug")]
        object_markers_button,
        #[cfg(feature = "debug")]
        light_markers_button,
        #[cfg(feature = "debug")]
        sound_markers_button,
        #[cfg(feature = "debug")]
        effect_markers_button,
        #[cfg(feature = "debug")]
        particle_markers_button,
        #[cfg(feature = "debug")]
        grid_text,
        #[cfg(feature = "debug")]
        tiles_button,
        #[cfg(feature = "debug")]
        pathing_button,
        #[cfg(feature = "debug")]
        buffers_text,
        #[cfg(feature = "debug")]
        diffuse_buffer_button,
        #[cfg(feature = "debug")]
        normal_buffer_button,
        #[cfg(feature = "debug")]
        depth_buffer_button,
    ];

    return window_builder.framed_window(interface_state, "render settings", elements, position);
}
