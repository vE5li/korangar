use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use cgmath::{Vector4, Zero};
use procedural::*;

use crate::debug::*;
use crate::graphics::{Color, InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::*;

#[derive(new)]
pub struct FrameInspectorView {
    #[new(default)]
    state: ElementState,
    measurement: Measurement,
    #[new(default)]
    start_offset: Duration,
    #[new(default)]
    end_offset: Duration,
}

impl Element for FrameInspectorView {
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, _theme: &Theme) {
        let size_constraint = &constraint!(100%, 300);
        self.state.resolve(placement_resolver, size_constraint);
    }

    fn hovered_element(&self, mouse_position: Position, mouse_mode: &MouseInputMode) -> HoverInformation {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position),
            _ => HoverInformation::Missed,
        }
    }

    fn scroll(&mut self, delta: f32) -> Option<ChangeEvent> {
        const ZOOM_SPEED: f32 = 0.004;

        let viewed_duration = self.measurement.total_time_taken() - (self.start_offset + self.end_offset);
        let side_bias = (1.0 / self.state.cached_size.x) * self.state.mouse_position.get().x;
        let total_offset = viewed_duration.mul_f32(delta.abs() * ZOOM_SPEED);

        if delta.is_sign_negative() {
            self.end_offset = self.end_offset.saturating_sub(total_offset.mul_f32(side_bias));
            self.start_offset = self.start_offset.saturating_sub(total_offset.mul_f32(1.0 - side_bias));
        } else {
            self.end_offset += total_offset.mul_f32(1.0 - side_bias);
            self.start_offset += total_offset.mul_f32(side_bias);
        }

        Some(ChangeEvent::RerenderWindow)
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        _state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        _theme: &Theme,
        parent_position: Position,
        clip_size: ClipSize,
        _hovered_element: Option<&dyn Element>,
        _focused_element: Option<&dyn Element>,
        _mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, clip_size);

        let mut colors = BTreeMap::new();

        fn render_measurement(
            renderer: &mut ElementRenderer<'_>,
            colors: &mut BTreeMap<&'static str, Color>,
            measurement: &Measurement,
            start_time: Instant,
            total_width: f32,
            unit: f32,
            y_position: f32,
        ) {
            let color = colors.entry(measurement.name).or_insert_with(|| {
                let [red, green, blue] = random_color::RandomColor::new().seed(measurement.name).to_rgb_array();
                Color::rgb(red, green, blue)
            });

            let x_position = measurement.start_time.saturating_duration_since(start_time).as_secs_f32() * unit + 1.0;
            let x_size = measurement.end_time.saturating_duration_since(start_time).as_secs_f32() * unit - x_position - 1.0;

            if x_size < 0.5 || x_position > total_width {
                return;
            }

            renderer.render_rectangle(
                Position::new(x_position, y_position),
                Size::new(x_size, 15.0),
                Vector4::new(0.0, 0.0, 0.0, 0.0),
                *color,
            );

            if x_size > 50.0 {
                let text = format!("{} ({:?})", measurement.name, measurement.end_time - measurement.start_time);
                renderer.render_text(&text, Position::new(x_position, y_position), Color::monochrome(0), 14.0);
            }

            measurement.indices.iter().for_each(|measurement| {
                render_measurement(renderer, colors, measurement, start_time, total_width, unit, y_position + 20.0)
            });
        }

        fn render_lines(renderer: &mut ElementRenderer<'_>, mut x_position: f32, distance: f32, size: Size) {
            while x_position < size.x {
                renderer.render_rectangle(
                    Position::new(x_position, 0.0),
                    Size::new(2.0, size.y),
                    Vector4::zero(),
                    Color::monochrome(65),
                );
                x_position += distance;
            }
        }

        let start_time = self.measurement.start_time + self.start_offset;
        let end_time = self.measurement.end_time - self.end_offset;
        let viewed_duration = end_time - start_time;

        if viewed_duration.as_millis() > 2 {
            let distance_per_millisecond = Duration::from_millis(1).div_duration_f32(viewed_duration) * self.state.cached_size.x;
            let offset = ((self.start_offset.as_micros() as f32 / 1000.0) * -distance_per_millisecond) % distance_per_millisecond;
            render_lines(&mut renderer, offset, distance_per_millisecond, self.state.cached_size);
        } else if viewed_duration.as_micros() > 50 {
            let distance_per_50_microseconds = Duration::from_micros(50).div_duration_f32(viewed_duration) * self.state.cached_size.x;
            let offset = ((self.start_offset.as_nanos() as f32 / 50000.0) * -distance_per_50_microseconds) % distance_per_50_microseconds;
            render_lines(&mut renderer, offset, distance_per_50_microseconds, self.state.cached_size);
        } else if viewed_duration.as_micros() > 2 {
            let distance_per_microsecond = Duration::from_micros(1).div_duration_f32(viewed_duration) * self.state.cached_size.x;
            let offset = ((self.start_offset.as_nanos() as f32 / 1000.0) * -distance_per_microsecond) % distance_per_microsecond;
            render_lines(&mut renderer, offset, distance_per_microsecond, self.state.cached_size);
        } else {
            let distance_per_50_nanoseconds = Duration::from_nanos(50).div_duration_f32(viewed_duration) * self.state.cached_size.x;
            let offset = ((self.start_offset.as_nanos() as f32 / 50.0) * -distance_per_50_nanoseconds) % distance_per_50_nanoseconds;
            render_lines(&mut renderer, offset, distance_per_50_nanoseconds, self.state.cached_size);
        }

        render_measurement(
            &mut renderer,
            &mut colors,
            &self.measurement,
            start_time,
            self.state.cached_size.x,
            self.state.cached_size.x / viewed_duration.as_secs_f32(),
            0.0,
        );
    }
}
