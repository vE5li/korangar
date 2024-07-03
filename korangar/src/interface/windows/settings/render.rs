use korangar_interface::elements::{ElementCell, ElementWrap, Expandable, StateButtonBuilder};
use korangar_interface::size_bound;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use rust_state::{Context, SafeUnwrap, Selector};

use crate::graphics::RenderSettings;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::GameState;

fn render_state_button(text: &'static str, selector: impl for<'a> Selector<'a, GameState, bool> + SafeUnwrap) -> ElementCell<GameState> {
    StateButtonBuilder::new()
        .with_text(text)
        .with_event(move |state: &Context<GameState>| {
            let current_value = *state.get_safe(&selector);
            state.update_value(&selector, !current_value);
            vec![]
        })
        .with_remote(selector)
        .build()
        .wrap()
}

fn general_expandable() -> ElementCell<GameState> {
    let buttons = vec![
        render_state_button("debug camera", RenderSettings::use_debug_camera(GameState::render_settings())),
        render_state_button("show fps", RenderSettings::show_frames_per_second(GameState::render_settings())),
        render_state_button("show wireframe", RenderSettings::show_wireframe(GameState::render_settings())),
        render_state_button("frustum culling", RenderSettings::frustum_culling(GameState::render_settings())),
        render_state_button(
            "show bounding boxes",
            RenderSettings::show_bounding_boxes(GameState::render_settings()),
        ),
    ];

    Expandable::new("general".to_string(), buttons, true).wrap()
}

fn map_expandable() -> ElementCell<GameState> {
    let buttons = vec![
        render_state_button("show map", RenderSettings::show_map(GameState::render_settings())),
        render_state_button("show objects", RenderSettings::show_objects(GameState::render_settings())),
        render_state_button("show entities", RenderSettings::show_entities(GameState::render_settings())),
        render_state_button("show water", RenderSettings::show_water(GameState::render_settings())),
        render_state_button("show indicators", RenderSettings::show_indicators(GameState::render_settings())),
    ];

    Expandable::new("map".to_string(), buttons, true).wrap()
}

fn lighting_expandable() -> ElementCell<GameState> {
    let buttons = vec![
        render_state_button(
            "ambient light",
            RenderSettings::show_ambient_light(GameState::render_settings()),
        ),
        render_state_button(
            "directional light",
            RenderSettings::show_directional_light(GameState::render_settings()),
        ),
        render_state_button("point lights", RenderSettings::show_point_lights(GameState::render_settings())),
        render_state_button(
            "particle lights",
            RenderSettings::show_particle_lights(GameState::render_settings()),
        ),
    ];

    Expandable::new("lighting".to_string(), buttons, true).wrap()
}

fn shadows_expandable() -> ElementCell<GameState> {
    let buttons = vec![render_state_button(
        "directional shadows",
        RenderSettings::show_directional_shadows(GameState::render_settings()),
    )];

    Expandable::new("shadows".to_string(), buttons, true).wrap()
}

fn markers_expandable() -> ElementCell<GameState> {
    let buttons = vec![
        render_state_button(
            "object markers",
            RenderSettings::show_object_markers(GameState::render_settings()),
        ),
        render_state_button(
            "light markers",
            RenderSettings::show_light_markers(GameState::render_settings()),
        ),
        render_state_button(
            "sound markers",
            RenderSettings::show_sound_markers(GameState::render_settings()),
        ),
        render_state_button(
            "effect markers",
            RenderSettings::show_effect_markers(GameState::render_settings()),
        ),
        render_state_button(
            "particle markers",
            RenderSettings::show_particle_markers(GameState::render_settings()),
        ),
        render_state_button(
            "entity markers",
            RenderSettings::show_entity_markers(GameState::render_settings()),
        ),
    ];

    Expandable::new("markers".to_string(), buttons, true).wrap()
}

fn grid_expandable() -> ElementCell<GameState> {
    let buttons = vec![
        render_state_button("map tiles", RenderSettings::show_map_tiles(GameState::render_settings())),
        render_state_button("pathing", RenderSettings::show_pathing(GameState::render_settings())),
    ];

    Expandable::new("grid".to_string(), buttons, true).wrap()
}

fn buffers_expandable() -> ElementCell<GameState> {
    let buttons = vec![
        render_state_button(
            "diffuse buffer",
            RenderSettings::show_diffuse_buffer(GameState::render_settings()),
        ),
        render_state_button(
            "normal buffer",
            RenderSettings::show_normal_buffer(GameState::render_settings()),
        ),
        render_state_button("water buffer", RenderSettings::show_water_buffer(GameState::render_settings())),
        render_state_button("depth buffer", RenderSettings::show_depth_buffer(GameState::render_settings())),
        render_state_button(
            "shadow buffer",
            RenderSettings::show_shadow_buffer(GameState::render_settings()),
        ),
        render_state_button(
            "picker buffer",
            RenderSettings::show_picker_buffer(GameState::render_settings()),
        ),
        render_state_button("font atlas", RenderSettings::show_font_atlas(GameState::render_settings())),
    ];

    Expandable::new("buffers".to_string(), buttons, true).wrap()
}

#[derive(Default)]
pub struct RenderSettingsWindow;

impl RenderSettingsWindow {
    pub const WINDOW_CLASS: &'static str = "render_settings";
}

impl PrototypeWindow<GameState> for RenderSettingsWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, application: &Context<GameState>, available_space: ScreenSize) -> Window<GameState> {
        let elements = vec![
            general_expandable(),
            map_expandable(),
            lighting_expandable(),
            shadows_expandable(),
            markers_expandable(),
            grid_expandable(),
            buffers_expandable(),
        ];

        WindowBuilder::new()
            .with_title("Render Settings".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(200 > 300 < 400, ?))
            .with_elements(elements)
            .closable()
            .build(window_cache, application, available_space)
    }
}
