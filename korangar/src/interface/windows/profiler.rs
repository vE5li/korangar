use std::cell::RefCell;

use korangar_debug::profiling::Profiler;
use korangar_interface::components::drop_down::DropDownItem;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{BaseLayoutInfo, Element, StateElement};
use korangar_interface::event::ClickHandler;
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{MouseButton, Resolver, WindowLayout};
use korangar_interface::prelude::EventQueue;
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Context, Path, RustState};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

// TODO: Move
pub mod color_lookup {
    use std::collections::BTreeMap;
    use std::hash::{DefaultHasher, Hash, Hasher};

    use crate::graphics::Color;

    const GOLDEN_RATIO_CONJUGATE: f64 = 0.618034;

    #[derive(Default)]
    pub struct ColorLookup {
        colors: BTreeMap<&'static str, Color>,
    }

    impl ColorLookup {
        pub fn get_color(&mut self, string: &'static str) -> Color {
            *self.colors.entry(string).or_insert_with(|| random_color(string))
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

struct DefaultClickHandler {
    inner: RefCell<(crate::threads::Enum, usize)>,
}

impl DefaultClickHandler {
    fn new() -> Self {
        Self {
            inner: RefCell::new((crate::threads::Enum::Main, 0)),
        }
    }

    fn update(&self, visible_thread: crate::threads::Enum, frame_index: usize) {
        *self.inner.borrow_mut() = (visible_thread, frame_index);
    }
}

impl ClickHandler<ClientState> for DefaultClickHandler {
    fn handle_click(&self, _: &Context<ClientState>, queue: &mut EventQueue<ClientState>) {
        let (visible_thread, frame_index) = *self.inner.borrow();

        let measurement = korangar_debug::profiling::get_frame_by_index(visible_thread, frame_index);

        queue.queue(InputEvent::InspectFrame { measurement });
    }
}

struct FrameView<A> {
    window_state_path: A,
    click_handler: DefaultClickHandler,
}

impl<A> FrameView<A> {
    pub fn new(window_state_path: A) -> Self {
        Self {
            window_state_path,
            click_handler: DefaultClickHandler::new(),
        }
    }
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
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let visible_thread = *state.get(&self.window_state_path.visible_thread());

        let (entries, longest_frame) = korangar_debug::profiling::get_frame_data(visible_thread);

        // TODO: We likely want to re-add gaps, it just looks bad right now.
        let gap_width = 0.0;
        let total_gaps = entries.len().saturating_sub(1) as f32 * gap_width;
        let bar_width = (layout_info.area.width - total_gaps) / entries.len() as f32;
        let height_unit = layout_info.area.height / longest_frame.as_secs_f32();
        let mut x_position = layout_info.area.left;
        let mut color_lookup = color_lookup::ColorLookup::default();

        layout.add_rectangle(
            layout_info.area,
            CornerDiameter::uniform(2.0),
            Color::monochrome_u8(40),
            Color::rgba_u8(0, 0, 0, 100),
            ShadowPadding::diagonal(2.0, 5.0),
        );

        for (index, entry) in entries.iter().enumerate() {
            let mut y_position = layout_info.area.top + layout_info.area.height;

            let area = Area {
                left: x_position,
                top: layout_info.area.top,
                width: bar_width + gap_width,
                height: layout_info.area.height,
            };

            let bar_height = height_unit * entry.total_time.as_secs_f32();
            let bar_area = Area {
                left: x_position,
                top: y_position - bar_height,
                width: bar_width,
                height: bar_height,
            };

            layout.add_rectangle(
                bar_area,
                CornerDiameter::default(),
                Color::monochrome_u8(80),
                Color::TRANSPARENT,
                ShadowPadding::uniform(0.0),
            );

            for (name, duration) in &entry.frame_times {
                let color = color_lookup.get_color(name);
                let bar_height = height_unit * duration.as_secs_f32();
                y_position -= bar_height;

                let bar_area = Area {
                    left: x_position,
                    top: y_position,
                    width: bar_width,
                    height: bar_height,
                };

                layout.add_rectangle(
                    bar_area,
                    CornerDiameter::default(),
                    color,
                    Color::TRANSPARENT,
                    ShadowPadding::uniform(0.0),
                );
            }

            if area.check().run(layout) {
                layout.add_rectangle(
                    bar_area,
                    CornerDiameter::default(),
                    Color::rgba_u8(255, 80, 0, 150),
                    Color::TRANSPARENT,
                    ShadowPadding::uniform(0.0),
                );

                self.click_handler.update(visible_thread, index);
                layout.register_click_handler(MouseButton::Left, &self.click_handler);
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
}

impl Default for ProfilerWindowState {
    fn default() -> Self {
        Self {
            visible_thread: crate::threads::Enum::Main,
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
        let halted = ComputedSelector::new_default(|_: &ClientState| Profiler::get_halted());

        window! {
            title: "Profiler",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        drop_down! {
                            selected: self.window_state_path.visible_thread(),
                            options: visible_thread_options.clone(),
                        },
                        state_button! {
                            text: "Halt",
                            state: halted,
                            event: |_: &Context<ClientState>, _: &mut EventQueue<ClientState>| {
                                let halted = Profiler::get_halted();
                                Profiler::set_halted(!halted);
                            },
                        },
                    ),
                },
                FrameView::new(self.window_state_path),
            ),
        }
    }
}
