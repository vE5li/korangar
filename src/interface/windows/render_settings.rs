use cgmath::Vector2;

use input::UserEvent;
use graphics::Color;

use super::super::*;

pub fn render_settings_window(window_builder: &mut WindowBuilder, interface_state: &mut InterfaceState) -> Element {

    let element_index = window_builder.unique_identifier();
    let position = Vector2::new(20.0, 20.0);
    let background_color = Color::new(5, 5, 5);
    let focused_background_color = Color::new(5, 5, 5);
    let text_offset = Vector2::new(10.0, 3.0);
    let text_color = Color::new(50, 50, 50);
    let font_size = 15.0;

    let frames_per_second_button = checkbutton(window_builder, String::from("frame counter"), UserEvent::ToggleShowFramesPerSecond, StateKey::ShowFramesPerSecond, window_builder.inner_width());
    let map_button = checkbutton(window_builder, String::from("map"), UserEvent::ToggleShowMap, StateKey::ShowMap, window_builder.inner_width());
    let objects_button = checkbutton(window_builder, String::from("objects"), UserEvent::ToggleShowObjects, StateKey::ShowObjects, window_builder.inner_width());
    let ambient_light_button = checkbutton(window_builder, String::from("ambient light"), UserEvent::ToggleShowAmbientLight, StateKey::ShowAmbientLight, window_builder.inner_width());
    let directional_light_button = checkbutton(window_builder, String::from("directional light"), UserEvent::ToggleShowDirectionalLight, StateKey::ShowDirectionalLight, window_builder.inner_width());
    let point_lights_button = checkbutton(window_builder, String::from("point lights"), UserEvent::ToggleShowPointLights, StateKey::ShowPointLights, window_builder.inner_width());
    let particle_lights_button = checkbutton(window_builder, String::from("particle lights"), UserEvent::ToggleShowParticleLights, StateKey::ShowParticleLights, window_builder.inner_width());
    #[cfg(feature = "debug")]
    let debug_camera_button = checkbutton(window_builder, String::from("debug camera"), UserEvent::ToggleUseDebugCamera, StateKey::UseDebugCamera, window_builder.inner_width());
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
    let tiles_button = checkbutton(window_builder, String::from("map tiles"), UserEvent::ToggleShowMapTiles, StateKey::ShowMapTiles, window_builder.inner_width());

    let elements = vec![
        frames_per_second_button,
        map_button,
        objects_button,
        ambient_light_button,
        directional_light_button,
        point_lights_button,
        particle_lights_button,
        #[cfg(feature = "debug")]
        debug_camera_button,
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
        tiles_button,
    ];

    let size = window_builder.final_size();
    let background = Component::Rectangle(RectangleComponent::new(size, background_color, focused_background_color));
    let text = Component::Text(TextComponent::new(text_offset, String::from("render settings"), text_color, font_size));
    let hoverable = Component::Hoverable(HoverableComponent::new(size));
    let draggable = Component::Draggable(DraggableComponent::new(interface_state));

    let container = Component::Container(ContainerComponent::new(elements));
    let components = vec![background, text, hoverable, draggable, container];

    return Element::new(components, element_index, position);
}
