use std::num::NonZeroU32;

use korangar_interface::elements::{ElementCell, ElementWrap, Expandable, PickList, StateButtonBuilder, Text};
use korangar_interface::state::{PlainTrackedState, TrackedStateBinary};
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_interface::{dimension_bound, size_bound};

use crate::graphics::RenderSettings;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;

fn render_state_button(text: &'static str, state: impl TrackedStateBinary<bool>) -> ElementCell<InterfaceSettings> {
    StateButtonBuilder::new()
        .with_text(text)
        .with_event(state.toggle_action())
        .with_remote(state.new_remote())
        .build()
        .wrap()
}

fn general_expandable(settings: &PlainTrackedState<RenderSettings>) -> ElementCell<InterfaceSettings> {
    let buttons = vec![
        render_state_button("debug camera", settings.mapped(|settings| &settings.use_debug_camera)),
        render_state_button("show fps", settings.mapped(|settings| &settings.show_frames_per_second)),
        render_state_button("show wireframe", settings.mapped(|settings| &settings.show_wireframe)),
        render_state_button("frustum culling", settings.mapped(|settings| &settings.frustum_culling)),
        render_state_button("show bounding boxes", settings.mapped(|settings| &settings.show_bounding_boxes)),
        render_state_button("show entities debug", settings.mapped(|settings| &settings.show_entities_debug)),
        render_state_button("show entities paper", settings.mapped(|settings| &settings.show_entities_paper)),
    ];

    Expandable::new("general".to_string(), buttons, true).wrap()
}

fn map_expandable(settings: &PlainTrackedState<RenderSettings>) -> ElementCell<InterfaceSettings> {
    let buttons = vec![
        render_state_button("show map", settings.mapped(|settings| &settings.show_map)),
        render_state_button("show objects", settings.mapped(|settings| &settings.show_objects)),
        render_state_button("show entities", settings.mapped(|settings| &settings.show_entities)),
        render_state_button("show water", settings.mapped(|settings| &settings.show_water)),
        render_state_button("show indicators", settings.mapped(|settings| &settings.show_indicators)),
    ];

    Expandable::new("map".to_string(), buttons, true).wrap()
}

fn lighting_expandable(settings: &PlainTrackedState<RenderSettings>) -> ElementCell<InterfaceSettings> {
    let buttons = vec![
        render_state_button("ambient light", settings.mapped(|settings| &settings.show_ambient_light)),
        render_state_button(
            "directional light",
            settings.mapped(|settings| &settings.show_directional_light),
        ),
        render_state_button("point lights", settings.mapped(|settings| &settings.show_point_lights)),
        render_state_button("particle lights", settings.mapped(|settings| &settings.show_particle_lights)),
    ];

    Expandable::new("lighting".to_string(), buttons, true).wrap()
}

fn markers_expandable(settings: &PlainTrackedState<RenderSettings>) -> ElementCell<InterfaceSettings> {
    let buttons = vec![
        render_state_button("object markers", settings.mapped(|settings| &settings.show_object_markers)),
        render_state_button("light markers", settings.mapped(|settings| &settings.show_light_markers)),
        render_state_button("sound markers", settings.mapped(|settings| &settings.show_sound_markers)),
        render_state_button("effect markers", settings.mapped(|settings| &settings.show_effect_markers)),
        render_state_button("particle markers", settings.mapped(|settings| &settings.show_particle_markers)),
        render_state_button("entity markers", settings.mapped(|settings| &settings.show_entity_markers)),
        render_state_button("shadow markers", settings.mapped(|settings| &settings.show_shadow_markers)),
    ];

    Expandable::new("markers".to_string(), buttons, true).wrap()
}

fn grid_expandable(settings: &PlainTrackedState<RenderSettings>) -> ElementCell<InterfaceSettings> {
    let buttons = vec![
        render_state_button("map tiles", settings.mapped(|settings| &settings.show_map_tiles)),
        render_state_button("pathing", settings.mapped(|settings| &settings.show_pathing)),
    ];

    Expandable::new("grid".to_string(), buttons, true).wrap()
}

fn buffers_expandable(settings: &PlainTrackedState<RenderSettings>) -> ElementCell<InterfaceSettings> {
    let setting_elements = vec![
        render_state_button("picker", settings.mapped(|settings| &settings.show_picker_buffer)),
        render_state_button(
            "directional shadow",
            settings.mapped(|settings| &settings.show_directional_shadow_map),
        ),
        Text::default().with_text("point shadow").with_width(dimension_bound!(50%)).wrap(),
        PickList::default()
            .with_options(vec![
                ("off", None),
                ("1", NonZeroU32::new(1)),
                ("2", NonZeroU32::new(2)),
                ("3", NonZeroU32::new(3)),
                ("4", NonZeroU32::new(4)),
                ("5", NonZeroU32::new(5)),
                ("6", NonZeroU32::new(6)),
            ])
            .with_selected(settings.mapped(|settings| &settings.show_point_shadow_map))
            .with_event(Box::new(Vec::new))
            .with_width(dimension_bound!(!))
            .wrap(),
        render_state_button(
            "light cull count",
            settings.mapped(|settings| &settings.show_light_culling_count_buffer),
        ),
        render_state_button("font atlas", settings.mapped(|settings| &settings.show_font_atlas)),
    ];

    Expandable::new("buffers".to_string(), setting_elements, true).wrap()
}

pub struct RenderSettingsWindow {
    render_settings: PlainTrackedState<RenderSettings>,
}

impl RenderSettingsWindow {
    pub const WINDOW_CLASS: &'static str = "render_settings";

    pub fn new(render_settings: PlainTrackedState<RenderSettings>) -> Self {
        Self { render_settings }
    }
}

impl PrototypeWindow<InterfaceSettings> for RenderSettingsWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let elements = vec![
            general_expandable(&self.render_settings),
            map_expandable(&self.render_settings),
            lighting_expandable(&self.render_settings),
            buffers_expandable(&self.render_settings),
            markers_expandable(&self.render_settings),
            grid_expandable(&self.render_settings),
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
