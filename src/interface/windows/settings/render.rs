use input::UserEvent;
use interface::traits::{ Window, PrototypeWindow };
use interface::types::InterfaceSettings;
use interface::elements::{ Expandable, StateButton };
use interface::{ StateProvider, WindowCache, FramedWindow, ElementCell, Size };

macro_rules! render_state_button {
    ($display:expr, $event:expr, $selector:ident) => {
        {
            let selector = Box::new(|state_provider: &StateProvider| state_provider.render_settings.$selector);
            cell!(StateButton::new($display, $event, selector))
        }
    };
}

fn general_expandable() -> ElementCell {

    let buttons: Vec<ElementCell> = vec![
        render_state_button!("debug camera", UserEvent::ToggleUseDebugCamera, use_debug_camera),
        render_state_button!("show fps", UserEvent::ToggleShowFramesPerSecond, show_frames_per_second),
    ];

    cell!(Expandable::new("general".to_string(), buttons, true))
}

fn map_expandable() -> ElementCell {

    let buttons: Vec<ElementCell> = vec![
        render_state_button!("show map", UserEvent::ToggleShowMap, show_map),
        render_state_button!("show objects", UserEvent::ToggleShowObjects, show_objects),
        render_state_button!("show entities", UserEvent::ToggleShowEntities, show_entities),
        render_state_button!("show water", UserEvent::ToggleShowWater, show_water),
    ];

    cell!(Expandable::new("map".to_string(), buttons, true))
}

fn lighting_expandable() -> ElementCell {

    let buttons: Vec<ElementCell> = vec![
        render_state_button!("ambient light", UserEvent::ToggleShowAmbientLight, show_ambient_light),
        render_state_button!("directional light", UserEvent::ToggleShowDirectionalLight, show_directional_light),
        render_state_button!("point lights", UserEvent::ToggleShowPointLights, show_point_lights),
        render_state_button!("particle lights", UserEvent::ToggleShowParticleLights, show_particle_lights),
    ];

    cell!(Expandable::new("lighting".to_string(), buttons, true))
}

fn shadows_expandable() -> ElementCell {

    let buttons: Vec<ElementCell> = vec![
        render_state_button!("directional shadows", UserEvent::ToggleShowDirectionalShadows, show_directional_shadows),
    ];

    cell!(Expandable::new("shadows".to_string(), buttons, true))
}

fn markers_expandable() -> ElementCell {

    let buttons: Vec<ElementCell> = vec![
        render_state_button!("object markers", UserEvent::ToggleShowObjectMarkers, show_object_markers),
        render_state_button!("light markers", UserEvent::ToggleShowLightMarkers, show_light_markers),
        render_state_button!("sound markers", UserEvent::ToggleShowSoundMarkers, show_sound_markers),
        render_state_button!("effect markers", UserEvent::ToggleShowEffectMarkers, show_effect_markers),
        render_state_button!("particle markers", UserEvent::ToggleShowParticleMarkers, show_particle_markers),
        render_state_button!("entity markers", UserEvent::ToggleShowEntityMarkers, show_entity_markers),
    ];

    cell!(Expandable::new("markers".to_string(), buttons, true))
}

fn grid_expandable() -> ElementCell {

    let buttons: Vec<ElementCell> = vec![
        render_state_button!("map tiles", UserEvent::ToggleShowMapTiles, show_map_tiles),
        render_state_button!("pathing", UserEvent::ToggleShowPathing, show_pathing),
    ];

    cell!(Expandable::new("grid".to_string(), buttons, true))
}

fn buffers_expandable() -> ElementCell {

    let buttons: Vec<ElementCell> = vec![
        render_state_button!("diffuse buffer", UserEvent::ToggleShowDiffuseBuffer, show_diffuse_buffer),
        render_state_button!("normal buffer", UserEvent::ToggleShowNormalBuffer, show_normal_buffer),
        render_state_button!("water buffer", UserEvent::ToggleShowWaterBuffer, show_water_buffer),
        render_state_button!("depth buffer", UserEvent::ToggleShowDepthBuffer, show_depth_buffer),
        render_state_button!("picker buffer", UserEvent::ToggleShowPickerBuffer, show_picker_buffer),
    ];

    cell!(Expandable::new("buffers".to_string(), buttons, true))
}

pub struct RenderSettingsWindow {
    window_class: String,
}

impl Default for RenderSettingsWindow {
   
    fn default() -> Self {
        Self { window_class: "render_settings".to_string() }
    }
}

impl PrototypeWindow for RenderSettingsWindow {

    fn window_class(&self) -> Option<&str> {
        Some(&self.window_class)
    } 

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, avalible_space: Size) -> Box<dyn Window + 'static> {

        let elements: Vec<ElementCell> = vec![
            general_expandable(),
            map_expandable(),
            lighting_expandable(),
            shadows_expandable(),
            markers_expandable(),
            grid_expandable(),
            buffers_expandable(),
        ];

        Box::from(FramedWindow::new(window_cache, interface_settings, avalible_space, "render settings".to_string(), self.window_class.clone().into(), elements, constraint!(200.0 > 250.0 < 300.0, ?)))
    }
}
