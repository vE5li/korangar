use std::num::NonZeroU32;

use korangar_interface::element::Element;
use korangar_interface::event::Toggle;
use korangar_interface::prelude::*;
use korangar_interface::window::{CustomWindow, PrototypeWindow, Window, WindowTrait};
use rust_state::{Context, Path, Selector};

use crate::graphics::{RenderSettings, RenderSettingsPathExt};
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::state::{ClientState, ClientThemeType};

fn render_state_button(text: &'static str, path: impl Path<ClientState, bool>) -> impl Element<ClientState> {
    state_button! {
        text: text,
        state: path,
        event: Toggle(path),
    }
}

fn general_expandable(path: impl Path<ClientState, RenderSettings>) -> impl Element<ClientState> {
    collapsable! {
        text: "general",
        initially_expanded: true,
        children: (
            render_state_button("debug camera", path.use_debug_camera()),
            render_state_button("show fps", path.show_frames_per_second()),
            render_state_button("show wireframe", path.show_wireframe()),
            render_state_button("frustum culling", path.frustum_culling()),
            render_state_button("show bounding boxes", path.show_bounding_boxes()),
            render_state_button("show entities debug", path.show_entities_debug()),
            render_state_button("show entities paper", path.show_entities_paper()),
        ),
    }
}

fn map_expandable(path: impl Path<ClientState, RenderSettings>) -> impl Element<ClientState> {
    collapsable! {
        text: "map",
        initially_expanded: true,
        children: (
            render_state_button("show map", path.show_map()),
            render_state_button("show objects", path.show_objects()),
            render_state_button("show entities", path.show_entities()),
            render_state_button("show water", path.show_water()),
            render_state_button("show indicators", path.show_indicators()),
        )
    }
}

fn lighting_expandable(path: impl Path<ClientState, RenderSettings>) -> impl Element<ClientState> {
    collapsable! {
        text: "lighting",
        initially_expanded: true,
        children: (
            render_state_button("ambient light", path.show_ambient_light()),
            render_state_button( "directional light", path.show_directional_light()),
            render_state_button("point lights", path.show_point_lights()),
            render_state_button("particle lights", path.show_particle_lights()),
        )
    }
}

fn markers_expandable(path: impl Path<ClientState, RenderSettings>) -> impl Element<ClientState> {
    collapsable! {
        text: "markers",
        initially_expanded: true,
        children: (
            render_state_button("object markers", path.show_object_markers()),
            render_state_button("light markers", path.show_light_markers()),
            render_state_button("sound markers", path.show_sound_markers()),
            render_state_button("effect markers", path.show_effect_markers()),
            render_state_button("particle markers", path.show_particle_markers()),
            render_state_button("entity markers", path.show_entity_markers()),
            render_state_button("shadow markers", path.show_shadow_markers()),
        )
    }
}

fn grid_expandable(path: impl Path<ClientState, RenderSettings>) -> impl Element<ClientState> {
    collapsable! {
        text: "grid",
        initially_expanded: true,
        children: (
            render_state_button("map tiles", path.show_map_tiles()),
            render_state_button("pathing", path.show_pathing()),
        )
    }
}

// fn buffers_expandable(path: impl Path<ClientState, RenderSettings>) -> impl
// Element<ClientState> {     collapsable! {
//         text: "buffers",
//         children: (
//             render_state_button("picker", path.show_picker_buffer()),
//             render_state_button("directional shadow",
// path.show_directional_shadow_map()),
// Text::default().with_text("point
// shadow").with_width(dimension_bound!(50%)).wrap(),
// PickList::default()                 .with_options(vec![
//                     ("off", None),
//                     ("1", NonZeroU32::new(1)),
//                     ("2", NonZeroU32::new(2)),
//                     ("3", NonZeroU32::new(3)),
//                     ("4", NonZeroU32::new(4)),
//                     ("5", NonZeroU32::new(5)),
//                     ("6", NonZeroU32::new(6)),
//                 ])
//                 .with_selected(path.show_point_shadow_map())
//                 .with_event(Box::new(Vec::new))
//                 .with_width(dimension_bound!(!))
//                 .wrap(),
//             render_state_button("light cull count",
// path.show_light_culling_count_buffer()),
// render_state_button("font map", path.show_font_map()),         )
//     }
// }

pub struct RenderSettingsWindow<P> {
    path: P,
}

impl<P> RenderSettingsWindow<P> {
    pub fn new(path: P) -> Self {
        Self { path }
    }
}

impl<P> CustomWindow<ClientState> for RenderSettingsWindow<P>
where
    P: Path<ClientState, RenderSettings>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::RenderSettings)
    }

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let elements = (
            general_expandable(self.path),
            map_expandable(self.path),
            lighting_expandable(self.path),
            // buffers_expandable(self.path),
            markers_expandable(self.path),
            grid_expandable(self.path),
        );

        window! {
            title: "Render Settings",
            class: Self::window_class(),
            theme: ClientThemeType::Game,
            closable: true,
            elements: (scroll_view! { children: elements, height_bound: HeightBound::WithMax, }, )
        }
    }
}
