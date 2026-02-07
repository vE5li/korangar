use std::cell::RefCell;
use std::time::Duration;

use korangar_debug::profiling::{FrameMeasurement, Measurement};
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{BaseLayoutInfo, Element};
use korangar_interface::event::{EventQueue, ScrollHandler};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::tooltip::TooltipExt;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::Context;

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::interface::windows::profiler::color_lookup::ColorLookup;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;
use crate::{FontSize, OverflowBehavior};

/// ZST for the tooltip id.
struct FrameInspectorTooltip;

const VISIBILITY_THRESHHOLD: f32 = 0.01;

struct MeasurementDetails {
    duration: String,
}

struct Inner {
    start_offset: Duration,
    end_offset: Duration,
    side_bias: f32,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            start_offset: Default::default(),
            end_offset: Default::default(),
            side_bias: 0.5,
        }
    }
}

struct FrameInspectorView {
    measurement: FrameMeasurement,
    measurement_details: Vec<MeasurementDetails>,
    inner: RefCell<Inner>,
}

impl FrameInspectorView {
    fn new(measurement: FrameMeasurement) -> Self {
        let measurement_details = measurement
            .measurements()
            .map(|measurement| MeasurementDetails {
                duration: format!("{:?}", measurement.total_time_taken()),
            })
            .collect();

        Self {
            measurement,
            measurement_details,
            inner: Default::default(),
        }
    }
}

impl FrameInspectorView {
    fn interpolate_alpha_linear(sloap: f32, start: f32, value: f32) -> f32 {
        f32::min(1.0, sloap * (value - start))
    }

    fn interpolate_alpha_smoothed(max: f32, min: f32, value: f32) -> f32 {
        if value < min {
            return 0.0;
        }

        // Clamp the value between 0 and 1 to ensure the interpolation is within bounds
        let normalized_value = num::clamp((value - min) / ((max - min) * 20.0), 0.0, 1.0);

        // Apply a smoothing function for smooth transitions
        1.0 - normalized_value.powf(0.4)
    }

    fn render_lines<'a>(
        &'a self,
        layout_info: &'a BaseLayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
        text: &'static str,
        mut x_position: f32,
        distance: f32,
        alpha: f32,
        render_numbers: bool,
    ) {
        let color = Color::rgb_u8(200, 200, 200).multiply_alpha(alpha);

        while x_position < layout_info.area.width {
            let line_width = 1.5;
            let line_area = Area {
                left: layout_info.area.left + x_position,
                top: layout_info.area.top,
                width: line_width,
                height: layout_info.area.height,
            };

            layout.add_rectangle(
                line_area,
                CornerDiameter::uniform(0.0),
                color,
                Color::TRANSPARENT,
                ShadowPadding::uniform(0.0),
            );

            if render_numbers {
                let text_area = Area {
                    left: layout_info.area.left + x_position,
                    top: layout_info.area.top + layout_info.area.height - 30.0,
                    width: distance,
                    height: 30.0,
                };

                layout.add_text(
                    text_area,
                    text,
                    FontSize(15.0),
                    Color::WHITE.multiply_alpha(alpha),
                    Color::rgb_u8(255, 160, 60),
                    HorizontalAlignment::Center { offset: 0.0, border: 3.0 },
                    VerticalAlignment::Center { offset: 0.0 },
                    OverflowBehavior::Shrink,
                );
            }

            x_position += distance;
        }
    }

    fn render_measurement<'a>(
        &'a self,
        layout_info: &'a BaseLayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
        color_lookup: &mut ColorLookup,
        measurement: &'a Measurement,
        measurement_details: &'a MeasurementDetails,
        start_time: std::time::Instant,
        unit: f32,
        y_position: f32,
    ) {
        const BAR_FADE_SPEED: f32 = 0.05;
        const TEXT_FADE_SPEED: f32 = 0.01;
        // Size in scaled pixels at which the text starts fading in
        const TEXT_DISPLAY_SIZE: f32 = 50.0;

        let scaled_bar_gap = 5.0;
        let color = color_lookup.get_color(measurement.name);
        let x_position = measurement.start_time.saturating_duration_since(start_time).as_secs_f32() * unit + scaled_bar_gap;
        let x_size = measurement.end_time.saturating_duration_since(start_time).as_secs_f32() * unit - x_position - scaled_bar_gap;
        let x_size = x_size.min(layout_info.area.width - x_position - scaled_bar_gap);
        let y_size = 20.0;

        let alpha = Self::interpolate_alpha_linear(BAR_FADE_SPEED, 0.0, x_size);
        if alpha < VISIBILITY_THRESHHOLD {
            return;
        }

        let block_area = Area {
            left: layout_info.area.left + x_position,
            top: y_position,
            width: x_size,
            height: y_size,
        };

        layout.add_rectangle(
            block_area,
            CornerDiameter::uniform(4.0),
            color.multiply_alpha(alpha),
            Color::TRANSPARENT,
            ShadowPadding::uniform(0.0),
        );

        if block_area.check().run(layout) {
            layout.add_rectangle(
                block_area,
                CornerDiameter::uniform(4.0),
                Color::rgba_u8(255, 255, 255, 100),
                Color::TRANSPARENT,
                ShadowPadding::uniform(0.0),
            );

            layout.add_tooltip(measurement.name, FrameInspectorTooltip.tooltip_id());
            layout.add_tooltip(&measurement_details.duration, FrameInspectorTooltip.tooltip_id());
        }

        let alpha = Self::interpolate_alpha_linear(TEXT_FADE_SPEED, TEXT_DISPLAY_SIZE, x_size);

        if alpha > VISIBILITY_THRESHHOLD {
            layout.add_text(
                block_area,
                measurement.name,
                FontSize(15.0),
                Color::BLACK.multiply_alpha(alpha),
                Color::rgb_u8(255, 160, 60),
                HorizontalAlignment::Left { offset: 3.0, border: 3.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );
        }

        let y_position = y_position + (scaled_bar_gap + y_size);

        measurement.indices.iter().for_each(|index| {
            self.render_measurement(
                layout_info,
                layout,
                color_lookup,
                &self.measurement[*index],
                &self.measurement_details[*index],
                start_time,
                unit,
                y_position,
            )
        });
    }
}

impl Element<ClientState> for FrameInspectorView {
    type LayoutInfo = BaseLayoutInfo;

    fn create_layout_info(
        &mut self,
        _: &Context<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let area = resolver.with_height(400.0);

            Self::LayoutInfo { area }
        })
    }

    fn lay_out<'a>(
        &'a self,
        _: &'a Context<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        layout.add_rectangle(
            layout_info.area,
            CornerDiameter::uniform(6.0),
            Color::monochrome_u8(60),
            Color::rgba_u8(0, 0, 0, 100),
            ShadowPadding::diagonal(2.0, 5.0),
        );

        if layout_info.area.check().dont_mark().run(layout) {
            let mut inner = self.inner.borrow_mut();

            // Technically side_bias should never be negative using this equation but since
            // a negative value results in an instant crash we make sure with a
            // `f32::abs`.
            inner.side_bias = f32::abs((1.0 / layout_info.area.width) * (layout.get_mouse_position().left - layout_info.area.left));

            layout.register_scroll_handler(self);
        }

        let mut colors = ColorLookup::default();

        let root_measurement = self.measurement.root_measurement();

        let inner = self.inner.borrow();

        let start_time = root_measurement.start_time + inner.start_offset;
        let end_time = root_measurement.end_time - inner.end_offset;
        let viewed_duration = end_time - start_time;

        // We only ever want to display a single unit, so we render from smallest to
        // biggest and keep track when a unit has been rendered.
        let mut numbers_shown = false;

        layout.with_clip(layout_info.area, |layout| {
            // Technically we could make this macro simpler by unifying the units used for
            // calculating visibility and offset, but that might introduce more
            // floating point inaccuracy as the values get very small or very large.
            macro_rules! render_lines_size {
                ($alpha_function:ident, $max:expr, $min:expr, $duration:expr, $div_function:ident, $divider:expr, $text:expr) => {
                    let visibility = Self::interpolate_alpha_smoothed($max, $min, viewed_duration.$alpha_function() as f32);

                    if visibility > VISIBILITY_THRESHHOLD {
                        let distance = $duration.div_duration_f32(viewed_duration) * layout_info.area.width;
                        let offset = ((inner.start_offset.$div_function() as f32 / $divider) * -distance) % distance;
                        let show_numbers = !numbers_shown;

                        #[allow(unused_assignments)]
                        {
                            numbers_shown |= show_numbers;
                        }

                        self.render_lines(layout_info, layout, $text, offset, distance, visibility, show_numbers);
                    }
                };
            }

            render_lines_size!(as_nanos, 20.0, 0.0, Duration::from_nanos(10), as_nanos, 10.0, "10ns");
            render_lines_size!(as_nanos, 200.0, 20.0, Duration::from_nanos(100), as_nanos, 100.0, "100ns");
            render_lines_size!(as_nanos, 2000.0, 200.0, Duration::from_micros(1), as_nanos, 1000.0, "1μs");
            render_lines_size!(as_micros, 20.0, 2.0, Duration::from_micros(10), as_nanos, 10000.0, "10μs");
            render_lines_size!(as_micros, 200.0, 20.0, Duration::from_micros(100), as_nanos, 100000.0, "100μs");
            render_lines_size!(as_micros, 2000.0, 200.0, Duration::from_millis(1), as_micros, 1000.0, "1ms");
            render_lines_size!(as_millis, 20.0, 2.0, Duration::from_millis(10), as_micros, 10000.0, "10ms");
            render_lines_size!(as_millis, 200.0, 20.0, Duration::from_millis(100), as_micros, 100000.0, "100ms");

            self.render_measurement(
                layout_info,
                layout,
                &mut colors,
                root_measurement,
                &self.measurement_details[0],
                start_time,
                layout_info.area.width / viewed_duration.as_secs_f32(),
                layout_info.area.top + 5.0,
            );
        });

        if layout_info.area.check().run(layout) {
            layout.add_tooltip("<none>", FrameInspectorTooltip.tooltip_id());
        }
    }
}

impl ScrollHandler<ClientState> for FrameInspectorView {
    fn handle_scroll(&self, _: &Context<ClientState>, _: &mut EventQueue<ClientState>, delta: f32) -> bool {
        const ZOOM_SPEED: f32 = 0.004;

        let mut inner = self.inner.borrow_mut();

        let root_measurement = self.measurement.root_measurement();
        let viewed_duration = root_measurement.total_time_taken() - (inner.start_offset + inner.end_offset);
        let total_offset = viewed_duration.mul_f32(delta.abs() * ZOOM_SPEED);
        let side_bias = inner.side_bias;

        if delta.is_sign_negative() {
            inner.end_offset = inner.end_offset.saturating_sub(total_offset.mul_f32(side_bias));
            inner.start_offset = inner.start_offset.saturating_sub(total_offset.mul_f32(1.0 - side_bias));
        } else {
            inner.end_offset += total_offset.mul_f32(1.0 - side_bias);
            inner.start_offset += total_offset.mul_f32(side_bias);
        }

        true
    }
}

pub struct FrameInspectorWindow {
    measurement: FrameMeasurement,
}

impl FrameInspectorWindow {
    pub fn new(measurement: FrameMeasurement) -> Self {
        Self { measurement }
    }
}

impl CustomWindow<ClientState> for FrameInspectorWindow {
    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Frame Inspector",
            theme: InterfaceThemeType::InGame,
            closable: true,
            minimum_width: 500.0,
            maximum_width: 2000.0,
            elements: (
                FrameInspectorView::new(self.measurement),
            ),
        }
    }
}
