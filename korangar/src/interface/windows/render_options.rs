use std::cell::UnsafeCell;
use std::num::NonZeroU32;

use korangar_interface::components::drop_down::DefaultClickHandler;
use korangar_interface::element::Element;
use korangar_interface::event::Toggle;
use korangar_interface::prelude::*;
use korangar_interface::window::{CustomWindow, StateWindow, Window, WindowTrait};
use rust_state::{Context, Path, Selector};

use crate::client_state;
use crate::graphics::{RenderOptions, RenderOptionsPathExt};
use crate::interface::layout::{ScreenSize, ScreenSizePathExt};
use crate::interface::windows::{WindowCache, WindowClass};
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt};

// TODO: Remove once event can be implied.
fn render_state_button(text: &'static str, path: impl Path<ClientState, bool>) -> impl Element<ClientState> {
    state_button! {
        text: text,
        state: path,
        event: Toggle(path),
    }
}

// TODO: Most likely move this.
pub struct ScalingSelector<S> {
    selector: S,
    scaling: f32,
    storage: UnsafeCell<f32>,
}

impl<S> ScalingSelector<S> {
    pub fn new(selector: S, scaling: f32) -> Self {
        Self {
            selector,
            scaling,
            storage: UnsafeCell::new(0.0),
        }
    }
}

impl<S> Selector<ClientState, f32, true> for ScalingSelector<S>
where
    S: Selector<ClientState, f32>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a f32> {
        // SAFETY
        //
        // It's safe to unwrap here because of the selector bound.
        let value = *self.selector.select(state).unwrap();

        let storage = unsafe { &mut *self.storage.get() };
        *storage = value * self.scaling;

        Some(storage)
    }
}

pub struct RenderOptionsWindow<P> {
    path: P,
}

impl<P> RenderOptionsWindow<P> {
    pub fn new(path: P) -> Self {
        Self { path }
    }
}

impl<P> CustomWindow<ClientState> for RenderOptionsWindow<P>
where
    P: Path<ClientState, RenderOptions>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::RenderOptions)
    }

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let elements = (
            collapsable! {
                text: "General",
                initially_expanded: true,
                children: (
                    render_state_button("Debug camera", self.path.use_debug_camera()),
                    render_state_button("Show fps", self.path.show_frames_per_second()),
                    render_state_button("Show wireframe", self.path.show_wireframe()),
                    render_state_button("Frustum culling", self.path.frustum_culling()),
                    render_state_button("Show bounding boxes", self.path.show_bounding_boxes()),
                    render_state_button("Show entities debug", self.path.show_entities_debug()),
                    render_state_button("Show entities paper", self.path.show_entities_paper()),
                ),
            },
            collapsable! {
                text: "Map",
                initially_expanded: true,
                children: (
                    render_state_button("Show map", self.path.show_map()),
                    render_state_button("Show objects", self.path.show_objects()),
                    render_state_button("Show entities", self.path.show_entities()),
                    render_state_button("Show water", self.path.show_water()),
                    render_state_button("Show indicators", self.path.show_indicators()),
                )
            },
            collapsable! {
                text: "Lighting",
                initially_expanded: true,
                children: (
                    render_state_button("Ambient light", self.path.show_ambient_light()),
                    render_state_button("Directional light", self.path.show_directional_light()),
                    render_state_button("Point lights", self.path.show_point_lights()),
                    render_state_button("Particle lights", self.path.show_particle_lights()),
                )
            },
            collapsable! {
                text: "Markers",
                initially_expanded: true,
                children: (
                    render_state_button("Object markers", self.path.show_object_markers()),
                    render_state_button("Light markers", self.path.show_light_markers()),
                    render_state_button("Sound markers", self.path.show_sound_markers()),
                    render_state_button("Effect markers", self.path.show_effect_markers()),
                    render_state_button("Particle markers", self.path.show_particle_markers()),
                    render_state_button("Entity markers", self.path.show_entity_markers()),
                    render_state_button("Shadow markers", self.path.show_shadow_markers()),
                )
            },
            collapsable! {
                text: "Grid",
                initially_expanded: true,
                children: (
                    render_state_button("Map tiles", self.path.show_map_tiles()),
                    render_state_button("Pathing", self.path.show_pathing()),
                )
            },
            collapsable! {
                text: "Buffers",
                initially_expanded: true,
                children: (
                    render_state_button("Picker", self.path.show_picker_buffer()),
                    render_state_button("Directional shadow", self.path.show_directional_shadow_map()),
                    // split! {
                    //     children: (
                    //         drop_down! {
                    //             selected: path.show_point_shadow_map(),
                    //             options: options,
                    //             click_handler: DefaultClickHandler::<_, _, _, ()>::new(path.show_point_shadow_map(), options),
                    //         }
                    //     ),
                    // },
                    render_state_button("Light cull count", self.path.show_light_culling_count_buffer()),
                    render_state_button("Font map", self.path.show_font_map()),
                )
            },
        );

        window! {
            title: "Render Options",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            closable: true,
            minimum_height: 300.0,
            // Set the maximum window height to be 80% of the main window height.
            maximum_height: ScalingSelector::new(client_state().window_size().height(), 0.8),
            elements: (scroll_view! { children: elements, height_bound: HeightBound::WithMax, }, )
        }
    }
}
