use procedural::*;

use crate::debug::*;
use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::*;

pub struct FrameView {
    state: ElementState,
    frame_counter: usize,
    always_update: Remote<bool>,
    visible_thread: Remote<ProfilerThread>,
}

impl FrameView {
    pub fn new(always_update: Remote<bool>, visible_thread: Remote<ProfilerThread>) -> Self {
        Self {
            state: ElementState::default(),
            frame_counter: 0,
            always_update,
            visible_thread,
        }
    }
}

impl Element for FrameView {
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, _theme: &InterfaceTheme) {
        let size_bound = &size_bound!(100%, 300);
        self.state.resolve(placement_resolver, size_bound);
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        self.frame_counter += 1;

        if *self.always_update.borrow() || self.frame_counter == SAVED_FRAME_COUNT {
            self.frame_counter = 0;
            return Some(ChangeEvent::RENDER_WINDOW);
        }

        None
    }

    fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position),
            _ => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _update: &mut bool) -> Vec<ClickAction> {
        let visible_thread = *self.visible_thread.borrow();
        let mouse_position = self.state.mouse_position.get();
        let number_of_frames = get_number_of_saved_frames(visible_thread);

        let bar_width = self.state.cached_size.width / number_of_frames as f32;
        let clicked_frame = (mouse_position.left / bar_width) as usize;

        let measurement = get_frame_by_index(visible_thread, clicked_frame);
        vec![ClickAction::OpenWindow(Box::new(FrameInspectorWindow::new(measurement)))]
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        _state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        _theme: &InterfaceTheme,
        parent_position: ScreenPosition,
        screen_clip: ScreenClip,
        _hovered_element: Option<&dyn Element>,
        _focused_element: Option<&dyn Element>,
        _mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, screen_clip);

        let (entries, statistics_map, longest_frame) = get_statistics_data(*self.visible_thread.borrow());

        let bar_width = (self.state.cached_size.width - 50.0) / entries.len() as f32;
        let gap_width = 50.0 / entries.len() as f32;
        let height_unit = self.state.cached_size.height / longest_frame.as_secs_f32();
        let mut x_position = 0.0;
        let mut color_lookup = super::ColorLookup::default();

        for entry in entries {
            let mut y_position = self.state.cached_size.height;

            let bar_height = height_unit * entry.total_time.as_secs_f32();
            let bar_position = ScreenPosition {
                left: x_position,
                top: y_position - bar_height,
            };
            let bar_size = ScreenSize {
                width: bar_width,
                height: bar_height,
            };

            renderer.render_rectangle(bar_position, bar_size, CornerRadius::default(), Color::monochrome_u8(80));

            for (name, duration) in entry.frame_times {
                let color = color_lookup.get_color(name);
                let bar_height = height_unit * duration.as_secs_f32();
                y_position -= bar_height;

                let bar_position = ScreenPosition {
                    left: x_position,
                    top: y_position,
                };
                let bar_size = ScreenSize {
                    width: bar_width,
                    height: bar_height,
                };

                renderer.render_rectangle(bar_position, bar_size, CornerRadius::default(), color);
            }

            x_position += bar_width + gap_width;
        }

        let mut y_position = 0.0;
        for (name, color) in std::iter::once((ROOT_MEASUREMENT_NAME, Color::monochrome_u8(150))).chain(color_lookup.into_iter()) {
            let statistics = statistics_map.get(name).unwrap();
            let text = format!("{} {:?} (SD {:.1})", name, statistics.mean, statistics.standard_deviation);

            let text_position = ScreenPosition {
                left: 3.0,
                top: y_position,
            };
            let shadow_position = text_position + ScreenSize::uniform(1.0);

            // Drop shadow.
            renderer.render_text(&text, shadow_position, Color::monochrome_u8(0), 14.0);
            // Colored text.
            renderer.render_text(&text, text_position, color, 14.0);

            y_position += 14.0;
        }
    }
}
