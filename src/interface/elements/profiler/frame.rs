use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Weak;

use cgmath::Array;
use procedural::*;

use crate::debug::*;
use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::input::{MouseInputMode, UserEvent};
use crate::interface::*;
use crate::network::CharacterInformation;

pub struct FrameViewer {
    state: ElementState,
    frame_counter: usize,
    always_update: Remote<bool>,
}

impl FrameViewer {
    pub fn new(always_update: Remote<bool>) -> Self {
        Self {
            state: ElementState::default(),
            frame_counter: 0,
            always_update,
        }
    }
}

impl Element for FrameViewer {
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, interface_settings: &InterfaceSettings, theme: &Theme) {
        let size_constraint = &constraint!(100%, 300);
        self.state.resolve(placement_resolver, size_constraint);
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        self.frame_counter += 1;

        if *self.always_update.borrow() || self.frame_counter == 127 {
            self.frame_counter = 0;
            return Some(ChangeEvent::RerenderWindow);
        }

        None
    }

    fn hovered_element(&self, mouse_position: Position, mouse_mode: &MouseInputMode) -> HoverInformation {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position),
            _ => HoverInformation::Missed,
        }
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &Theme,
        parent_position: Position,
        clip_size: ClipSize,
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        mouse_mode: &MouseInputMode,
        second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, clip_size);

        let (entries, statistics_map, longest_frame) = get_statistics_data();

        let mut colors = BTreeMap::new();
        let bar_width = (self.state.cached_size.x - 50.0) / entries.len() as f32;
        let gap_width = 50.0 / entries.len() as f32;
        let height_unit = self.state.cached_size.y / longest_frame.as_secs_f32();
        let mut x_position = 0.0;

        for entry in entries {
            let mut y_position = self.state.cached_size.y;

            let bar_height = height_unit * entry.total_time.as_secs_f32();

            renderer.render_rectangle(
                Position::new(x_position, y_position - bar_height),
                Size::new(bar_width, bar_height),
                cgmath::Vector4::new(0.0, 0.0, 0.0, 0.0),
                Color::monochrome(80),
            );

            for (name, duration) in entry.frame_times {
                let bar_height = height_unit * duration.as_secs_f32();
                y_position -= bar_height;

                let color = colors.entry(name).or_insert_with(|| {
                    let [red, green, blue] = random_color::RandomColor::new().seed(name).to_rgb_array();
                    Color::rgb(red, green, blue)
                });

                renderer.render_rectangle(
                    Position::new(x_position, y_position),
                    Size::new(bar_width, bar_height),
                    cgmath::Vector4::new(0.0, 0.0, 0.0, 0.0),
                    *color,
                );
            }

            x_position += bar_width + gap_width;
        }

        let mut y_position = 0.0;
        for (name, color) in std::iter::once((&MAIN_EVENT_MEASUREMENT_NAME, &Color::monochrome(150))).chain(colors.iter()) {
            let statistics = statistics_map.get(name).unwrap();
            let text = format!("{} {:?} (SD {:.1})", name, statistics.mean, statistics.standard_deviation);

            // Drop shadow.
            renderer.render_text(&text, Position::new(4.0, y_position + 1.0), Color::monochrome(0), 14.0);
            // Colored text.
            renderer.render_text(&text, Position::new(3.0, y_position), *color, 14.0);

            y_position += 14.0;
        }
    }
}
