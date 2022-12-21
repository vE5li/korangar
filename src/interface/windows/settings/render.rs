use procedural::*;

use crate::input::UserEvent;
use crate::interface::*;

macro render_state_button($display:expr, $event:expr, $selector:ident) {
    StateButton::default()
        .with_static_text($display)
        .with_selector(|state_provider| state_provider.render_settings.$selector)
        .with_event($event)
        .wrap()
}

fn general_expandable() -> ElementCell {
    let buttons: Vec<ElementCell> = vec![
        render_state_button!("debug camera", UserEvent::ToggleUseDebugCamera, use_debug_camera),
        render_state_button!("show fps", UserEvent::ToggleShowFramesPerSecond, show_frames_per_second),
        render_state_button!("show wireframe", UserEvent::ToggleShowWireframe, show_wireframe),
        render_state_button!("frustum culling", UserEvent::ToggleFrustumCulling, frustum_culling),
        render_state_button!("show bounding boxes", UserEvent::ToggleShowBoundingBoxes, show_bounding_boxes),
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
        render_state_button!(
            "directional light",
            UserEvent::ToggleShowDirectionalLight,
            show_directional_light
        ),
        render_state_button!("point lights", UserEvent::ToggleShowPointLights, show_point_lights),
        render_state_button!("particle lights", UserEvent::ToggleShowParticleLights, show_particle_lights),
    ];

    cell!(Expandable::new("lighting".to_string(), buttons, true))
}

fn shadows_expandable() -> ElementCell {
    let buttons: Vec<ElementCell> = vec![render_state_button!(
        "directional shadows",
        UserEvent::ToggleShowDirectionalShadows,
        show_directional_shadows
    )];

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
        render_state_button!("shadow buffer", UserEvent::ToggleShowShadowBuffer, show_shadow_buffer),
        render_state_button!("picker buffer", UserEvent::ToggleShowPickerBuffer, show_picker_buffer),
        render_state_button!("font atlas", UserEvent::ToggleShowFontAtlas, show_font_atlas),
    ];

    cell!(Expandable::new("buffers".to_string(), buttons, true))
}

#[derive(Default)]
pub struct RenderSettingsWindow {}

impl RenderSettingsWindow {
    pub const WINDOW_CLASS: &'static str = "render_settings";
}

impl PrototypeWindow for RenderSettingsWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: Size) -> Window {
        let elements: Vec<ElementCell> = vec![
            general_expandable(),
            map_expandable(),
            lighting_expandable(),
            shadows_expandable(),
            markers_expandable(),
            grid_expandable(),
            buffers_expandable(),
        ];

        WindowBuilder::default()
            .with_title("Render Settings".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size(constraint!(200 > 250 < 300, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, interface_settings, available_space)
    }
}
