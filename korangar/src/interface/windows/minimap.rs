use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, State};

use crate::interface::minimap::MinimapView;
use crate::loaders::OverflowBehavior;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, MinimapState, MinimapStatePathExt};
use crate::world::Entity;

use super::WindowClass;

pub struct MinimapWindow<A, B> {
    minimap_path: A,
    entities_path: B,
}

impl<A, B> MinimapWindow<A, B> {
    pub fn new(minimap_path: A, entities_path: B) -> Self {
        Self {
            minimap_path,
            entities_path,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for MinimapWindow<A, B>
where
    A: Path<ClientState, MinimapState> + Clone + 'static,
    B: Path<ClientState, Vec<Entity>> + Clone + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Minimap)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        const MIN_ZOOM: f32 = 1.0;
        const MAX_ZOOM: f32 = 4.0;
        const ZOOM_STEP: f32 = 0.5;

        let zoom_out_path = self.minimap_path;
        let zoom_text_path = self.minimap_path;
        let zoom_in_path = self.minimap_path;

        window! {
            title: "Minimap",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            minimum_width: 240.0,
            maximum_width: 360.0,
            minimum_height: 290.0,
            maximum_height: 290.0,
            closable: true,
            elements: (
                split! {
                    children: (
                        button! {
                            text: "-",
                            event: move |state: &State<ClientState>, _: &mut EventQueue<ClientState>| {
                                state.update_value_with(zoom_out_path.zoom(), |zoom| {
                                    *zoom = (*zoom - ZOOM_STEP).clamp(MIN_ZOOM, MAX_ZOOM);
                                });
                            },
                        },
                        text! {
                            text: ComputedSelector::new_default(move |state: &ClientState| {
                                format!("Zoom {:.1}x", zoom_text_path.follow_safe(state).zoom)
                            }),
                            horizontal_alignment: HorizontalAlignment::Center { offset: 0.0, border: 4.0 },
                            vertical_alignment: VerticalAlignment::Center { offset: 0.0 },
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        button! {
                            text: "+",
                            event: move |state: &State<ClientState>, _: &mut EventQueue<ClientState>| {
                                state.update_value_with(zoom_in_path.zoom(), |zoom| {
                                    *zoom = (*zoom + ZOOM_STEP).clamp(MIN_ZOOM, MAX_ZOOM);
                                });
                            },
                        },
                    ),
                },
                MinimapView::new(self.minimap_path, self.entities_path),
            ),
        }
    }
}
