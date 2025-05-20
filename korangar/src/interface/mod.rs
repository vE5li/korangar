pub mod layout;
pub mod theme;
#[macro_use]
pub mod elements;
pub mod application;
pub mod cursor;
// pub mod dialog;
pub mod linked;
pub mod resource;
pub mod windows;

pub mod components {
    pub mod character_slot_preview {
        use std::cell::UnsafeCell;
        use std::fmt::Display;

        use korangar_interface::element::Element;
        use korangar_interface::element::id::ElementIdGenerator;
        use korangar_interface::element::store::ElementStore;
        use korangar_interface::event::{ClickAction, EventQueue};
        use korangar_interface::layout::alignment::{HorizontalAlignment, VerticalAlignment};
        use korangar_interface::layout::{Layout, Resolver};
        use ragnarok_packets::{CharacterInformation, CharacterInformationPathExt};
        use rust_state::{Context, ManuallyAssertExt, Path, RustState, Selector};

        use crate::graphics::Color;
        use crate::input::UserEvent;
        use crate::interface::layout::CornerRadius;
        use crate::loaders::FontSize;
        use crate::state::ClientState;

        #[derive(RustState)]
        pub struct CharacterSlotPreviewTheme {
            pub background_color: ClientState,
        }

        pub struct CharacterSlotPreview<P, A> {
            pub path: P,
            pub background_color: A,
            pub click_handler: CharacterSlotPreviewHandler,
            pub slot: usize,
        }

        impl<P, A> Element<ClientState> for CharacterSlotPreview<P, A>
        where
            P: Path<ClientState, CharacterInformation, false>,
            A: Selector<ClientState, Color>,
        {
            fn get_height(&self, state: &Context<ClientState>, _: &ElementStore, _: &mut ElementIdGenerator, resolver: &mut Resolver) {
                resolver.with_height(180.0);
            }

            fn create_layout<'a>(
                &'a self,
                state: &'a Context<ClientState>,
                _: &'a ElementStore,
                _: &mut ElementIdGenerator,
                resolver: &mut Resolver,
                layout: &mut Layout<'a, ClientState>,
            ) {
                let area = resolver.with_height(180.0);

                let is_hoverered = layout.is_area_hovered_and_active(area);

                layout.add_rectangle(area, CornerRadius::uniform(2.0), *state.get(&self.background_color));

                if let Some(character_information) = state.try_get(&self.path) {
                    layout.add_text(
                        area,
                        &character_information.name,
                        FontSize(18.0),
                        Color::rgb_u8(255, 200, 150),
                        HorizontalAlignment::Left { offset: 5.0 },
                        VerticalAlignment::Top { offset: 0.0 },
                    );

                    layout.add_text(
                        area,
                        "Base level",
                        FontSize(14.0),
                        Color::rgb_u8(200, 200, 150),
                        HorizontalAlignment::Right { offset: 5.0 },
                        VerticalAlignment::Top { offset: 0.0 },
                    );

                    layout.add_text(
                        area,
                        self.click_handler
                            .base_level_str
                            .get_str(self.path.manually_asserted().base_level(), state),
                        FontSize(14.0),
                        Color::rgb_u8(200, 200, 150),
                        HorizontalAlignment::Right { offset: 5.0 },
                        VerticalAlignment::Top { offset: 14.0 },
                    );

                    layout.add_text(
                        area,
                        "Job level",
                        FontSize(14.0),
                        Color::rgb_u8(200, 200, 150),
                        HorizontalAlignment::Right { offset: 5.0 },
                        VerticalAlignment::Top { offset: 36.0 },
                    );

                    layout.add_text(
                        area,
                        self.click_handler
                            .job_level_str
                            .get_str(self.path.manually_asserted().job_level(), state),
                        FontSize(14.0),
                        Color::rgb_u8(200, 200, 150),
                        HorizontalAlignment::Right { offset: 5.0 },
                        VerticalAlignment::Top { offset: 50.0 },
                    );

                    layout.add_text(
                        area,
                        "Map",
                        FontSize(14.0),
                        Color::rgb_u8(200, 200, 150),
                        HorizontalAlignment::Right { offset: 5.0 },
                        VerticalAlignment::Top { offset: 72.0 },
                    );

                    layout.add_text(
                        area,
                        &character_information.map_name,
                        FontSize(14.0),
                        Color::rgb_u8(200, 200, 150),
                        HorizontalAlignment::Right { offset: 5.0 },
                        VerticalAlignment::Top { offset: 86.0 },
                    );
                } else {
                    layout.add_text(
                        area,
                        "Create Character",
                        FontSize(14.0),
                        Color::WHITE,
                        HorizontalAlignment::Center { offset: 0.0 },
                        VerticalAlignment::Center { offset: 0.0 },
                    );
                }

                if is_hoverered {
                    layout.add_click_area(area, &self.click_handler.on_click);
                    layout.mark_hovered();
                }
            }
        }

        struct PartialEqDisplayStr<T> {
            last_value: UnsafeCell<Option<T>>,
            text: UnsafeCell<String>,
        }

        impl<T> PartialEqDisplayStr<T> {
            pub fn new() -> Self {
                Self {
                    last_value: UnsafeCell::default(),
                    text: UnsafeCell::default(),
                }
            }
        }

        impl<T> PartialEqDisplayStr<T>
        where
            T: Clone + PartialEq + Display + 'static,
        {
            fn get_str<'a, P>(&'a self, path: P, state: &'a Context<ClientState>) -> &'a str
            where
                P: Path<ClientState, T>,
            {
                // SAFETY
                // `unnwrap` is safe here because the bound of `P` specifies a safe path.
                let value = state.get(&path);

                unsafe {
                    let last_value = &mut *self.last_value.get();

                    if last_value.is_none() || last_value.as_ref().is_some_and(|last| last != value) {
                        *self.text.get() = value.to_string();
                        *last_value = Some(value.clone());
                    }
                }

                unsafe { self.text.as_ref_unchecked() }
            }
        }

        struct OnClick {
            slot: usize,
        }

        impl ClickAction<ClientState> for OnClick {
            fn execute(&self, _: &Context<ClientState>, queue: &mut EventQueue<ClientState>) {
                queue.queue(UserEvent::SelectCharacter { slot: self.slot });
            }
        }

        pub struct CharacterSlotPreviewHandler {
            on_click: OnClick,
            base_level_str: PartialEqDisplayStr<i16>,
            job_level_str: PartialEqDisplayStr<i32>,
        }

        impl CharacterSlotPreviewHandler {
            pub fn new(slot: usize) -> Self {
                Self {
                    on_click: OnClick { slot },
                    base_level_str: PartialEqDisplayStr::new(),
                    job_level_str: PartialEqDisplayStr::new(),
                }
            }
        }
    }
}
