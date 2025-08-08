use korangar_interface::components::drop_down::{DefaultClickHandler, DropDownItem};
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{BaseLayoutInfo, Element, StateElement};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Layout, Resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Context, Path, RustState};

use crate::graphics::Color;
use crate::interface::layout::CornerRadius;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

// TODO: Move
mod color_lookup {
    use std::collections::{BTreeMap, btree_map};
    use std::hash::{DefaultHasher, Hash, Hasher};

    use crate::graphics::Color;

    const GOLDEN_RATIO_CONJUGATE: f64 = 0.618034;

    #[derive(Default)]
    pub(super) struct ColorLookup {
        colors: BTreeMap<&'static str, Color>,
    }

    impl ColorLookup {
        pub fn get_color(&mut self, string: &'static str) -> Color {
            *self.colors.entry(string).or_insert_with(|| random_color(string))
        }

        pub fn into_iter(self) -> btree_map::IntoIter<&'static str, Color> {
            self.colors.into_iter()
        }
    }

    fn random_color(string: &str) -> Color {
        let mut hasher = DefaultHasher::new();
        string.hash(&mut hasher);
        let hash = hasher.finish();

        let hue_base = (hash as f64) / (u64::MAX as f64);
        let saturation_value = ((hash >> 8) & 0xFFFF) as u32;
        let brightness_value = ((hash >> 24) & 0xFFFF) as u32;

        // Rotate by golden ratio conjugate for pleasing distribution
        let hue = (((hue_base + GOLDEN_RATIO_CONJUGATE) % 1.0) * 360.0) as u32;

        // Map saturation to a pleasing range (75-90)
        let saturation = 75 + (saturation_value % 15);

        // Map brightness to a pleasing range (85-100)
        let brightness = 85 + (brightness_value % 15);

        hsb_to_rgb(hue, saturation, brightness)
    }

    fn hsb_to_rgb(mut hue: u32, saturation: u32, brightness: u32) -> Color {
        if hue == 0 {
            hue = 1
        }

        if hue == 360 {
            hue = 359
        }

        let h: f32 = hue as f32 / 360.0;
        let s: f32 = saturation as f32 / 100.0;
        let b: f32 = brightness as f32 / 100.0;

        let h_i = (h * 6.0).floor();
        let f = h * 6.0 - h_i;
        let p = b * (1.0 - s);
        let q = b * (1.0 - f * s);
        let t = b * (1.0 - (1.0 - f) * s);

        let (r, g, b) = match h_i as i64 {
            0 => (b, t, p),
            1 => (q, b, p),
            2 => (p, b, t),
            3 => (p, q, b),
            4 => (t, p, b),
            _ => (b, p, q),
        };

        Color::rgb(r, g, b)
    }
}

struct FrameView<A> {
    window_state_path: A,
}

impl<A> Element<ClientState> for FrameView<A>
where
    A: Path<ClientState, ProfilerWindowState>,
{
    type LayoutInfo = BaseLayoutInfo;

    fn create_layout_info(
        &mut self,
        _: &Context<ClientState>,
        _: ElementStoreMut<'_>,
        resolver: &mut Resolver<'_, ClientState>,
    ) -> Self::LayoutInfo {
        let area = resolver.with_height(200.0);
        Self::LayoutInfo { area }
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut Layout<'a, ClientState>,
    ) {
        let visible_thread = *state.get(&self.window_state_path.visible_thread());

        let (entries, statistics_map, longest_frame) = korangar_debug::profiling::get_statistics_data(visible_thread);

        let gap_width = 1.0;
        let total_gaps = entries.len().saturating_sub(1) as f32 * gap_width;
        let bar_width = (layout_info.area.width - total_gaps) / entries.len() as f32;
        let height_unit = layout_info.area.height / longest_frame.as_secs_f32();
        let mut x_position = layout_info.area.left;
        let mut color_lookup = color_lookup::ColorLookup::default();

        layout.add_rectangle(layout_info.area, CornerRadius::uniform(2.0), Color::monochrome_u8(40));

        for entry in entries {
            let mut y_position = layout_info.area.top + layout_info.area.height;

            let bar_height = height_unit * entry.total_time.as_secs_f32();
            let bar_area = Area {
                left: x_position,
                top: y_position - bar_height,
                width: bar_width,
                height: bar_height,
            };

            layout.add_rectangle(bar_area, CornerRadius::default(), Color::monochrome_u8(80));

            let hover_area = Area {
                left: x_position,
                top: y_position - bar_height,
                width: bar_width + gap_width,
                height: bar_height,
            };

            let is_hovered = layout.is_area_hovered_and_active(hover_area);

            if is_hovered {
                layout.mark_hovered();
            }

            for (name, duration) in entry.frame_times {
                let color = color_lookup.get_color(name);
                let bar_height = height_unit * duration.as_secs_f32();
                y_position -= bar_height;

                let bar_area = Area {
                    left: x_position,
                    top: y_position,
                    width: bar_width,
                    height: bar_height,
                };

                layout.add_rectangle(bar_area, CornerRadius::default(), color);
            }

            x_position += bar_width + gap_width;
        }
    }
}

impl DropDownItem<crate::threads::Enum> for crate::threads::Enum {
    fn text(&self) -> &str {
        match self {
            crate::threads::Enum::Main => "Main thread",
            crate::threads::Enum::Loader => "Loader thread",
        }
    }

    fn value(&self) -> crate::threads::Enum {
        *self
    }
}

/// Internal state of the chat window.
#[derive(RustState, StateElement)]
pub struct ProfilerWindowState {
    visible_thread: crate::threads::Enum,
    halted: bool,
}

impl Default for ProfilerWindowState {
    fn default() -> Self {
        Self {
            visible_thread: crate::threads::Enum::Main,
            halted: false,
        }
    }
}

pub struct ProfilerWindow<A> {
    window_state_path: A,
}

impl<A> ProfilerWindow<A> {
    pub fn new(window_state_path: A) -> Self {
        Self { window_state_path }
    }
}

impl<A> CustomWindow<ClientState> for ProfilerWindow<A>
where
    A: Path<ClientState, ProfilerWindowState>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Profiler)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let visible_thread_options = vec![crate::threads::Enum::Main, crate::threads::Enum::Loader];

        window! {
            title: "Profiler",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            closable: true,
            elements: (
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        drop_down! {
                            selected: self.window_state_path.visible_thread(),
                            options: visible_thread_options.clone(),
                            click_handler: DefaultClickHandler::new(self.window_state_path.visible_thread(), visible_thread_options.clone()),
                        },
                        state_button! {
                            text: "Halt",
                            state: self.window_state_path.halted(),
                            event: Toggle(self.window_state_path.halted()),
                        },
                    ),
                },
                FrameView {
                    window_state_path: self.window_state_path,
                },
            ),
        }
    }
}
