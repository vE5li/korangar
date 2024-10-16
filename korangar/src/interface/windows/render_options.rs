use std::num::NonZeroU32;

use korangar_interface::window::{CustomWindow, Window};
use rust_state::Path;

use crate::graphics::{RenderOptions, RenderOptionsPathExt};
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

pub struct RenderOptionsWindow<P> {
    render_options_path: P,
}

impl<P> RenderOptionsWindow<P> {
    pub fn new(render_options_path: P) -> Self {
        Self { render_options_path }
    }
}

impl<P> CustomWindow<ClientState> for RenderOptionsWindow<P>
where
    P: Path<ClientState, RenderOptions>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::RenderOptions)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let show_point_shadow_map_options = vec![
            None,
            NonZeroU32::new(1),
            NonZeroU32::new(2),
            NonZeroU32::new(3),
            NonZeroU32::new(4),
            NonZeroU32::new(5),
            NonZeroU32::new(6),
        ];

        let elements = (
            collapsable! {
                text: "General",
                tooltip: "General tools for checking and debugging the rendering",
                initially_expanded: true,
                children: (
                    state_button! {
                        text: "Debug camera",
                        tooltip: "Enable the floating ^000001debug camera^000000. Change the view direction by holding the ^000001right mouse button^000000 and move around using ^000001W^000000, ^000001A^000000, ^000001S^000000, and ^000001D^000000. Hold ^000001shift^000000 to accelerate.",
                        state: self.render_options_path.use_debug_camera(),
                        event: Toggle(self.render_options_path.use_debug_camera()),
                    },
                    state_button! {
                        text: "Show fps",
                        tooltip: "Show the most recent ^000001frames per second^000000",
                        state: self.render_options_path.show_frames_per_second(),
                        event: Toggle(self.render_options_path.show_frames_per_second()),
                    },
                    state_button! {
                        text: "Show wireframe",
                        tooltip: "Show ^000001geometry^000000 as ^000001wireframe^000000",
                        state: self.render_options_path.show_wireframe(),
                        event: Toggle(self.render_options_path.show_wireframe()),
                    },
                    state_button! {
                        text: "Frustum culling",
                        tooltip: "^000001Discard geometry^000000 early if it is not in the ^000001view frustum^000000",
                        state: self.render_options_path.frustum_culling(),
                        event: Toggle(self.render_options_path.frustum_culling()),
                    },
                    state_button! {
                        text: "Show bounding boxes",
                        tooltip: "Show all object ^000001bounding boxes^000000. Bounding boxes of ^000001culled objects^000000 are ^000001purple^000000 and ^000001visible^000000 ones are ^000001yellow^000000.",
                        state: self.render_options_path.show_bounding_boxes(),
                        event: Toggle(self.render_options_path.show_bounding_boxes()),
                    },
                    state_button! {
                        text: "Show entities debug",
                        tooltip: "",
                        state: self.render_options_path.show_entities_debug(),
                        event: Toggle(self.render_options_path.show_entities_debug()),
                    },
                    state_button! {
                        text: "Show entities paper",
                        tooltip: "Always ^000001billboard^000000 entities towards the ^000001player camera^000000. This only has an effect when viewed with the ^000001debug camera^000000.",
                        state: self.render_options_path.show_entities_paper(),
                        event: Toggle(self.render_options_path.show_entities_paper()),
                    },
                ),
            },
            collapsable! {
                text: "World",
                tooltip: "Tools for checking and debugging the ^000001world rendering^000000",
                initially_expanded: true,
                children: (
                    state_button! {
                        text: "Show map",
                        tooltip: "Show the ^000001map geometry^000000",
                        state: self.render_options_path.show_map(),
                        event: Toggle(self.render_options_path.show_map()),
                    },
                    state_button! {
                        text: "Show objects",
                        tooltip: "Show ^000001objects^000000",
                        state: self.render_options_path.show_objects(),
                        event: Toggle(self.render_options_path.show_objects()),
                    },
                    state_button! {
                        text: "Show entities",
                        tooltip: "Show ^000001entities^000000. This includes ^000001players^000000, ^000001monsters^000000, ^000001NPCs^000000, and ^000001warps^000000",
                        state: self.render_options_path.show_entities(),
                        event: Toggle(self.render_options_path.show_entities()),
                    },
                    state_button! {
                        text: "Show water",
                        tooltip: "Show the ^000001water plane^000000",
                        state: self.render_options_path.show_water(),
                        event: Toggle(self.render_options_path.show_water()),
                    },
                    state_button! {
                        text: "Show indicators",
                        tooltip: "Show the ^000001tile indicator^000000 when hovering a walkable tile",
                        state: self.render_options_path.show_indicators(),
                        event: Toggle(self.render_options_path.show_indicators()),
                    },
                ),
            },
            collapsable! {
                text: "Lighting",
                tooltip: "Tools for checking and debugging the ^000001lighting^000000",
                initially_expanded: true,
                children: (
                    state_button! {
                        text: "Ambient lighting",
                        tooltip: "Enable ^000001ambient lighting^000000",
                        state: self.render_options_path.enable_ambient_lighting(),
                        event: Toggle(self.render_options_path.enable_ambient_lighting()),
                    },
                    state_button! {
                        text: "Directional lighting",
                        tooltip: "Enable ^000001directional lighting^000000",
                        state: self.render_options_path.enable_directional_lighting(),
                        event: Toggle(self.render_options_path.enable_directional_lighting()),
                    },
                    state_button! {
                        text: "Point lights",
                        tooltip: "Enable ^000001point lights^000000",
                        state: self.render_options_path.enable_point_lights(),
                        event: Toggle(self.render_options_path.enable_point_lights()),
                    },
                    state_button! {
                        text: "Particle lighting",
                        tooltip: "Enable ^000001particle lighting^000000",
                        state: self.render_options_path.enable_particle_lighting(),
                        event: Toggle(self.render_options_path.enable_particle_lighting()),
                    },
                ),
            },
            collapsable! {
                text: "Markers",
                tooltip: "^000001Markers^000000 for checking the position of ^000001resources^000000 and ^000001entites^000000 in the world",
                initially_expanded: true,
                children: (
                    state_button! {
                        text: "Object markers",
                        tooltip: "Show ^000001object markers^000000. The markers can be ^000001clicked^000000 to show ^000001more details^000000.",
                        state: self.render_options_path.show_object_markers(),
                        event: Toggle(self.render_options_path.show_object_markers()),
                    },
                    state_button! {
                        text: "Light markers",
                        tooltip: "Show ^000001light markers^000000. The markers can be ^000001clicked^000000 to show ^000001more details^000000.",
                        state: self.render_options_path.show_light_markers(),
                        event: Toggle(self.render_options_path.show_light_markers()),
                    },
                    state_button! {
                        text: "Sound markers",
                        tooltip: "Show ^000001sound markers^000000. The markers can be ^000001clicked^000000 to show ^000001more details^000000.",
                        state: self.render_options_path.show_sound_markers(),
                        event: Toggle(self.render_options_path.show_sound_markers()),
                    },
                    state_button! {
                        text: "Effect markers",
                        tooltip: "Show ^000001effect markers^000000. The markers can be ^000001clicked^000000 to show ^000001more details^000000.",
                        state: self.render_options_path.show_effect_markers(),
                        event: Toggle(self.render_options_path.show_effect_markers()),
                    },
                    state_button! {
                        text: "Particle markers",
                        tooltip: "Show ^000001particle markers^000000",
                        state: self.render_options_path.show_particle_markers(),
                        event: Toggle(self.render_options_path.show_particle_markers()),
                    },
                    state_button! {
                        text: "Entity markers",
                        tooltip: "Show ^000001entity markers^000000. The markers can be ^000001clicked^000000 to show ^000001more details^000000.",
                        state: self.render_options_path.show_entity_markers(),
                        event: Toggle(self.render_options_path.show_entity_markers()),
                    },
                    state_button! {
                        text: "Shadow markers",
                        tooltip: "Show ^000001shadow markers^000000",
                        state: self.render_options_path.show_shadow_markers(),
                        event: Toggle(self.render_options_path.show_shadow_markers()),
                    },
                ),
            },
            collapsable! {
                text: "Grid",
                tooltip: "Tools for checking and debugging the map ^000001tile grid^000000",
                initially_expanded: true,
                children: (
                    state_button! {
                        text: "Show map tiles",
                        tooltip: "Show the ^000001map tiles^000000. The mesh is rendered slightly higher to not collide with the ground vertices. Only tiles that are either ^000001walkable^000000, ^000001water^000000, ^000001snipable^000000, or ^000001cliff^000000 are rendered.",
                        state: self.render_options_path.show_map_tiles(),
                        event: Toggle(self.render_options_path.show_map_tiles()),
                    },
                    state_button! {
                        text: "Show pathing",
                        tooltip: "Show ^000001entity pathing^000000. This includes pathing of ^000001players^000000, ^000001monsters^000000, and ^000001NPCs^000000. The color of the path depends on the ^000001entity type^000000.",
                        state: self.render_options_path.show_pathing(),
                        event: Toggle(self.render_options_path.show_pathing()),
                    },
                ),
            },
            collapsable! {
                text: "Interface",
                tooltip: "Tools for checking and debugging the behavior of the ^000001user interface^000000",
                initially_expanded: true,
                children: (
                    state_button! {
                        text: "Show rectangle instructions",
                        tooltip: "Show all ^000001rectangle instructions^000000. The color of each rectangle is ^000001varied based on height^000000 to make overlapping rectangles easier to distinguish. Rectangles that are ^000001culled^000000 are displayed ^000001red^000000.",
                        state: self.render_options_path.show_rectangle_instructions(),
                        event: Toggle(self.render_options_path.show_rectangle_instructions()),
                    },
                    state_button! {
                        text: "Show glyph instructions",
                        tooltip: "Show all ^000001glyph instructions^000000. Glyphs that are ^000001culled^000000 are displayed ^000001red^000000.",
                        state: self.render_options_path.show_glyph_instructions(),
                        event: Toggle(self.render_options_path.show_glyph_instructions()),
                    },
                    state_button! {
                        text: "Show sprite instructions",
                        tooltip: "Show all ^000001sprite instructions^000000. Sprites that are ^000001culled^000000 are displayed ^000001red^000000.",
                        state: self.render_options_path.show_sprite_instructions(),
                        event: Toggle(self.render_options_path.show_sprite_instructions()),
                    },
                    state_button! {
                        text: "Show SDF instructions",
                        tooltip: "Show all ^000001signed distance field instructions^000000. SDFs that are ^000001culled^000000 are displayed ^000001red^000000.",
                        state: self.render_options_path.show_sdf_instructions(),
                        event: Toggle(self.render_options_path.show_sdf_instructions()),
                    },
                    state_button! {
                        text: "Show click areas",
                        tooltip: "Show all ^000001click areas^000000. In most cases click areas should only exist for ^000001hovered components^000000.",
                        state: self.render_options_path.show_click_areas(),
                        event: Toggle(self.render_options_path.show_click_areas()),
                    },
                    state_button! {
                        text: "Show drop areas",
                        tooltip: "Show all ^000001drop areas^000000. In most cases drop areas should only exist for ^000001hovered components^000000.",
                        state: self.render_options_path.show_drop_areas(),
                        event: Toggle(self.render_options_path.show_drop_areas()),
                    },
                    state_button! {
                        text: "Show scroll areas",
                        tooltip: "Show all ^000001scroll areas^000000. In most cases scroll areas should only exist for ^000001hovered components^000000.",
                        state: self.render_options_path.show_scroll_areas(),
                        event: Toggle(self.render_options_path.show_scroll_areas()),
                    },
                ),
            },
            collapsable! {
                text: "Buffers",
                tooltip: "Tools for checking and debugging ^000001buffers^000000 used for rendering and mouse input",
                initially_expanded: true,
                children: (
                    state_button! {
                        text: "Picker",
                        tooltip: "Overlay the ^000001picker buffer^000000",
                        state: self.render_options_path.show_picker_buffer(),
                        event: Toggle(self.render_options_path.show_picker_buffer()),
                    },
                    state_button! {
                        text: "Directional shadow",
                        tooltip: "Overlay the ^000001directional shadow map^000000",
                        state: self.render_options_path.show_directional_shadow_map(),
                        event: Toggle(self.render_options_path.show_directional_shadow_map()),
                    },
                    split! {
                        children: (
                            text! {
                                text: "Point shadow"
                            },
                            drop_down! {
                                selected: self.render_options_path.show_point_shadow_map(),
                                options: show_point_shadow_map_options.clone(),
                                click_handler: DefaultClickHandler::new(self.render_options_path.show_point_shadow_map(), show_point_shadow_map_options.clone()),
                            },
                        ),
                    },
                    state_button! {
                        text: "Light cull count",
                        tooltip: "Overlay the ^000001light culling count buffer^000000",
                        state: self.render_options_path.show_light_culling_count_buffer(),
                        event: Toggle(self.render_options_path.show_light_culling_count_buffer()),
                    },
                    state_button! {
                        text: "Font map",
                        tooltip: "Overlay the ^000001font map^000000",
                        state: self.render_options_path.show_font_map(),
                        event: Toggle(self.render_options_path.show_font_map()),
                    },
                ),
            },
        );

        window! {
            title: "Render Options",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            closable: true,
            resizable: true,
            minimum_height: 300.0,
            maximum_height: 900.0,
            elements: (scroll_view! { children: elements }, )
        }
    }
}
