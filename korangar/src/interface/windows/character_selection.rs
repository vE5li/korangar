use character_slot_preview::{CharacterSlotPreview, CharacterSlotPreviewHandler, OverlayHandler};
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{BaseLayoutInfo, Element};
use korangar_interface::layout::{Resolver, WindowLayout};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Context, Path};

use crate::character_slots::{CharacterSlots, CharacterSlotsExt};
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

mod character_slot_preview {
    use std::cell::UnsafeCell;
    use std::fmt::Display;

    use korangar_interface::element::store::{ElementStore, ElementStoreMut};
    use korangar_interface::element::{BaseLayoutInfo, Element};
    use korangar_interface::event::{ClickHandler, EventQueue};
    use korangar_interface::layout::alignment::{HorizontalAlignment, VerticalAlignment};
    use korangar_interface::layout::tooltip::TooltipExt;
    use korangar_interface::layout::{MouseButton, Resolver, WindowLayout};
    use ragnarok_packets::{CharacterInformation, CharacterInformationPathExt};
    use rust_state::{Context, ManuallyAssertExt, Path};

    use crate::graphics::{Color, CornerDiameter, ScreenPosition, ScreenSize, ShadowPadding};
    use crate::input::InputEvent;
    use crate::loaders::{FontSize, OverflowBehavior};
    use crate::state::ClientState;

    pub struct OverlayHandler<A, B> {
        position: ScreenPosition,
        size: ScreenSize,
        slot: usize,
        switch_request_path: A,
        character_information_path: B,
        window_id: u64,
    }

    impl<A, B> OverlayHandler<A, B> {
        pub fn new(slot: usize, switch_request_path: A, character_information_path: B) -> Self {
            Self {
                position: ScreenPosition { left: 0.0, top: 0.0 },
                size: ScreenSize { width: 0.0, height: 0.0 },
                slot,
                switch_request_path,
                character_information_path,
                window_id: 0,
            }
        }

        fn set_position_size(&mut self, position: ScreenPosition, size: ScreenSize, window_id: u64) {
            self.position = position;
            self.size = size;
            self.window_id = window_id;
        }
    }

    impl<A, B> ClickHandler<ClientState> for OverlayHandler<A, B>
    where
        A: Path<ClientState, Option<usize>>,
        B: Path<ClientState, CharacterInformation, false>,
    {
        fn execute(&self, _: &Context<ClientState>, queue: &mut EventQueue<ClientState>) {
            use korangar_interface::prelude::*;

            let slot = self.slot;
            let switch_request_path = self.switch_request_path;
            let character_information_path = self.character_information_path;

            let element = ErasedElement::new(fragment! {
                gaps: 4.0,
                children: (
                    button! {
                        text: "Delete",
                        event: move |state: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
                            // SAFETY
                            // We should not be able to get here if the character is not present, so it's
                            // fine to unwrap.
                            let character_information = state.try_get(&character_information_path).unwrap();
                            let character_id = character_information.character_id;

                            queue.queue(InputEvent::DeleteCharacter { character_id });
                            queue.queue(Event::CloseOverlay);
                        },
                    },
                    button! {
                        text: "Switch",
                        event: move |state: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
                            state.update_value(switch_request_path, Some(slot));
                            queue.queue(Event::CloseOverlay);
                        },
                    },
                    button! {
                        text: "Cancel",
                        event: move |_: &Context<ClientState>, queue: &mut EventQueue<ClientState>| {
                            queue.queue(Event::CloseOverlay);
                        },
                    },
                ),
            });

            queue.queue(Event::OpenOverlay {
                element,
                position: self.position,
                size: self.size,
                window_id: self.window_id,
            });
        }
    }

    // #[derive(RustState)]
    // pub struct CharacterSlotPreviewTheme {
    //     pub background_color: ClientState,
    // }

    pub struct CharacterSlotPreview<P, M, B> {
        character_information: P,
        switch_request: M,
        click_handler: CharacterSlotPreviewHandler<B>,
        overlay_handler: OverlayHandler<M, P>,
        slot: usize,
    }

    impl<P, M, B> CharacterSlotPreview<P, M, B> {
        pub fn new(
            character_information: P,
            switch_request: M,
            click_handler: CharacterSlotPreviewHandler<B>,
            overlay_handler: OverlayHandler<M, P>,
            slot: usize,
        ) -> Self {
            Self {
                character_information,
                switch_request,
                click_handler,
                overlay_handler,
                slot,
            }
        }
    }

    impl<P, M, B> Element<ClientState> for CharacterSlotPreview<P, M, B>
    where
        P: Path<ClientState, CharacterInformation, false>,
        M: Path<ClientState, Option<usize>>,
        B: Path<ClientState, Option<usize>>,
    {
        type LayoutInfo = BaseLayoutInfo;

        fn create_layout_info(
            &mut self,
            _: &Context<ClientState>,
            store: ElementStoreMut<'_>,
            resolver: &mut Resolver<'_, ClientState>,
        ) -> Self::LayoutInfo {
            let area = resolver.with_height(180.0);

            self.overlay_handler.set_position_size(
                ScreenPosition {
                    left: area.left,
                    top: area.top,
                },
                ScreenSize {
                    width: area.width,
                    height: area.height,
                },
                store.get_window_id(),
            );

            Self::LayoutInfo { area }
        }

        fn lay_out<'a>(
            &'a self,
            state: &'a Context<ClientState>,
            _: ElementStore<'a>,
            layout_info: &'a Self::LayoutInfo,
            layout: &mut WindowLayout<'a, ClientState>,
        ) {
            if let Some(switch_request) = state.get(&self.switch_request) {
                let is_hoverered = layout_info.area.check().run(layout);

                let background_color = match is_hoverered {
                    true => Color::monochrome_u8(80),
                    false => Color::monochrome_u8(60),
                };
                layout.add_rectangle(
                    layout_info.area,
                    CornerDiameter::uniform(25.0),
                    background_color,
                    Color::rgba_u8(0, 0, 0, 100),
                    ShadowPadding::diagonal(2.0, 5.0),
                );

                if *switch_request == self.slot {
                    layout.add_text(
                        layout_info.area,
                        "Cancel",
                        FontSize(14.0),
                        Color::WHITE,
                        Color::rgb_u8(255, 160, 60),
                        HorizontalAlignment::Center { offset: 0.0, border: 5.0 },
                        VerticalAlignment::Center { offset: 0.0 },
                        OverflowBehavior::Shrink,
                    );

                    if is_hoverered {
                        layout.add_click_area(layout_info.area, MouseButton::Left, &self.click_handler.cancel_switch);
                    }
                } else {
                    layout.add_text(
                        layout_info.area,
                        "Switch slots",
                        FontSize(14.0),
                        Color::WHITE,
                        Color::rgb_u8(255, 160, 60),
                        HorizontalAlignment::Center { offset: 0.0, border: 5.0 },
                        VerticalAlignment::Center { offset: 0.0 },
                        OverflowBehavior::Shrink,
                    );

                    if is_hoverered {
                        layout.add_click_area(layout_info.area, MouseButton::Left, &self.click_handler.request_switch);
                    }
                }

                return;
            }

            if let Some(character_information) = state.try_get(&self.character_information) {
                let is_hoverered = layout_info.area.check().run(layout);

                let background_color = match is_hoverered {
                    true => Color::monochrome_u8(110),
                    false => Color::monochrome_u8(90),
                };
                layout.add_rectangle(
                    layout_info.area,
                    CornerDiameter::uniform(25.0),
                    background_color,
                    Color::rgba_u8(0, 0, 0, 100),
                    ShadowPadding::diagonal(2.0, 5.0),
                );

                layout.add_text(
                    layout_info.area,
                    &character_information.name,
                    FontSize(18.0),
                    Color::rgb_u8(255, 200, 150),
                    Color::rgb_u8(255, 160, 60),
                    HorizontalAlignment::Center { offset: 0.0, border: 5.0 },
                    VerticalAlignment::Top { offset: 0.0 },
                    OverflowBehavior::Shrink,
                );

                layout.add_text(
                    layout_info.area,
                    "Base level",
                    FontSize(14.0),
                    Color::rgb_u8(200, 200, 150),
                    Color::rgb_u8(255, 160, 60),
                    HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                    VerticalAlignment::Top { offset: 30.0 },
                    OverflowBehavior::Shrink,
                );

                layout.add_text(
                    layout_info.area,
                    self.click_handler
                        .base_level_str
                        .get_str(self.character_information.manually_asserted().base_level(), state),
                    FontSize(14.0),
                    Color::rgb_u8(200, 200, 150),
                    Color::rgb_u8(255, 160, 60),
                    HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                    VerticalAlignment::Top { offset: 44.0 },
                    OverflowBehavior::Shrink,
                );

                layout.add_text(
                    layout_info.area,
                    "Job level",
                    FontSize(14.0),
                    Color::rgb_u8(200, 200, 150),
                    Color::rgb_u8(255, 160, 60),
                    HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                    VerticalAlignment::Top { offset: 66.0 },
                    OverflowBehavior::Shrink,
                );

                layout.add_text(
                    layout_info.area,
                    self.click_handler
                        .job_level_str
                        .get_str(self.character_information.manually_asserted().job_level(), state),
                    FontSize(14.0),
                    Color::rgb_u8(200, 200, 150),
                    Color::rgb_u8(255, 160, 60),
                    HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                    VerticalAlignment::Top { offset: 80.0 },
                    OverflowBehavior::Shrink,
                );

                layout.add_text(
                    layout_info.area,
                    "Map",
                    FontSize(14.0),
                    Color::rgb_u8(200, 200, 150),
                    Color::rgb_u8(255, 160, 60),
                    HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                    VerticalAlignment::Top { offset: 102.0 },
                    OverflowBehavior::Shrink,
                );

                layout.add_text(
                    layout_info.area,
                    // TODO: Replace with a map name lookup
                    character_information
                        .map_name
                        .strip_suffix(".gat")
                        .unwrap_or(&character_information.map_name),
                    FontSize(14.0),
                    Color::rgb_u8(200, 200, 150),
                    Color::rgb_u8(255, 160, 60),
                    HorizontalAlignment::Left { offset: 5.0, border: 3.0 },
                    VerticalAlignment::Top { offset: 116.0 },
                    OverflowBehavior::Shrink,
                );

                if is_hoverered {
                    layout.add_click_area(layout_info.area, MouseButton::Left, &self.click_handler.select_character);
                    layout.add_click_area(layout_info.area, MouseButton::Right, &self.overlay_handler);

                    {
                        struct PrivateTooltipId;
                        layout.add_tooltip(&character_information.name, PrivateTooltipId.tooltip_id());
                    }
                }
            } else {
                let is_hoverered = layout_info.area.check().run(layout);

                let background_color = match is_hoverered {
                    true => Color::monochrome_u8(55),
                    false => Color::monochrome_u8(40),
                };
                layout.add_rectangle(
                    layout_info.area,
                    CornerDiameter::uniform(25.0),
                    background_color,
                    Color::rgba_u8(0, 0, 0, 100),
                    ShadowPadding::diagonal(2.0, 5.0),
                );

                layout.add_text(
                    layout_info.area,
                    "Create Character",
                    FontSize(14.0),
                    Color::monochrome_u8(85),
                    Color::rgb_u8(255, 160, 60),
                    HorizontalAlignment::Center { offset: 0.0, border: 5.0 },
                    VerticalAlignment::Center { offset: 0.0 },
                    OverflowBehavior::Shrink,
                );

                if is_hoverered {
                    layout.add_click_area(layout_info.area, MouseButton::Left, &self.click_handler.create_character);
                }
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

    struct SelectCharacter {
        slot: usize,
    }

    impl ClickHandler<ClientState> for SelectCharacter {
        fn execute(&self, _: &Context<ClientState>, queue: &mut EventQueue<ClientState>) {
            queue.queue(InputEvent::SelectCharacter { slot: self.slot });
        }
    }

    struct CreateCharacter {
        slot: usize,
    }

    impl ClickHandler<ClientState> for CreateCharacter {
        fn execute(&self, _: &Context<ClientState>, queue: &mut EventQueue<ClientState>) {
            queue.queue(InputEvent::OpenCharacterCreationWindow { slot: self.slot });
        }
    }

    struct CancelSwitch<P> {
        switch_request: P,
    }

    impl<P> ClickHandler<ClientState> for CancelSwitch<P>
    where
        P: Path<ClientState, Option<usize>>,
    {
        fn execute(&self, state: &Context<ClientState>, _: &mut EventQueue<ClientState>) {
            state.update_value(self.switch_request, None);
        }
    }

    struct RequestSwitch<P> {
        switch_request: P,
        slot: usize,
    }

    impl<P> ClickHandler<ClientState> for RequestSwitch<P>
    where
        P: Path<ClientState, Option<usize>>,
    {
        fn execute(&self, state: &Context<ClientState>, queue: &mut EventQueue<ClientState>) {
            // SAFETY
            // We should not be able to get here if there is no switch request, so it's
            // fine to unwrap.
            let origin_slot = state.get(&self.switch_request).unwrap();

            queue.queue(InputEvent::SwitchCharacterSlot {
                origin_slot,
                destination_slot: self.slot,
            });
        }
    }

    pub struct CharacterSlotPreviewHandler<P> {
        select_character: SelectCharacter,
        create_character: CreateCharacter,
        cancel_switch: CancelSwitch<P>,
        request_switch: RequestSwitch<P>,
        base_level_str: PartialEqDisplayStr<i16>,
        job_level_str: PartialEqDisplayStr<i32>,
    }

    impl<P> CharacterSlotPreviewHandler<P>
    where
        P: Path<ClientState, Option<usize>>,
    {
        pub fn new(switch_request: P, slot: usize) -> Self {
            Self {
                select_character: SelectCharacter { slot },
                create_character: CreateCharacter { slot },
                cancel_switch: CancelSwitch { switch_request },
                request_switch: RequestSwitch { switch_request, slot },
                base_level_str: PartialEqDisplayStr::new(),
                job_level_str: PartialEqDisplayStr::new(),
            }
        }
    }
}

pub struct CharacterSelectionWindow<C, M> {
    character_slots: C,
    switch_request: M,
}

impl<C, M> CharacterSelectionWindow<C, M> {
    pub fn new(characters: C, switch_request: M) -> Self {
        Self {
            character_slots: characters,
            switch_request,
        }
    }
}

impl<C, M> CustomWindow<ClientState> for CharacterSelectionWindow<C, M>
where
    C: Path<ClientState, CharacterSlots>,
    M: Path<ClientState, Option<usize>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::CharacterSelection)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        type RowLayoutInfo = (BaseLayoutInfo, BaseLayoutInfo, BaseLayoutInfo, BaseLayoutInfo, BaseLayoutInfo);

        struct CharacterWrapper<C, M> {
            character_slots: C,
            switch_request: M,
            item_boxes: Vec<Box<dyn Element<ClientState, LayoutInfo = RowLayoutInfo>>>,
        }

        impl<C, M> CharacterWrapper<C, M>
        where
            C: Path<ClientState, CharacterSlots>,
            M: Path<ClientState, Option<usize>>,
        {
            fn new(character_slots: C, switch_request: M) -> Self {
                Self {
                    character_slots,
                    switch_request,
                    item_boxes: Vec::new(),
                }
            }

            fn correct_element_size(&mut self, state: &Context<ClientState>) {
                let character_slots = state.get(&self.character_slots);
                let slot_count = character_slots.get_slot_count();

                // FIX: Very broken check
                if self.item_boxes.len() != slot_count / 5 {
                    self.item_boxes.clear();

                    for row in 0..slot_count / 5 {
                        let slot = row * 5;
                        let path = self.character_slots;

                        self.item_boxes.push(Box::new(split! {
                            gaps: 10.0,
                            children: (
                                CharacterSlotPreview::new(
                                    path.in_slot(slot),
                                    self.switch_request,
                                    CharacterSlotPreviewHandler::new(self.switch_request, slot),
                                    OverlayHandler::new(slot, self.switch_request, path.in_slot(slot)),
                                    slot,
                                ),
                                CharacterSlotPreview::new(
                                    path.in_slot(slot + 1),
                                    self.switch_request,
                                    CharacterSlotPreviewHandler::new(self.switch_request, slot + 1),
                                    OverlayHandler::new(slot + 1, self.switch_request, path.in_slot(slot + 1)),
                                    slot + 1,
                                ),
                                CharacterSlotPreview::new(
                                    path.in_slot(slot + 2),
                                    self.switch_request,
                                    CharacterSlotPreviewHandler::new(self.switch_request, slot + 2),
                                    OverlayHandler::new(slot + 2, self.switch_request, path.in_slot(slot + 2)),
                                    slot + 2,
                                ),
                                CharacterSlotPreview::new(
                                    path.in_slot(slot + 3),
                                    self.switch_request,
                                    CharacterSlotPreviewHandler::new(self.switch_request, slot + 3),
                                    OverlayHandler::new(slot + 3, self.switch_request, path.in_slot(slot + 3)),
                                    slot + 3,
                                ),
                                CharacterSlotPreview::new(
                                    path.in_slot(slot + 4),
                                    self.switch_request,
                                    CharacterSlotPreviewHandler::new(self.switch_request, slot + 4),
                                    OverlayHandler::new(slot + 4, self.switch_request, path.in_slot(slot + 4)),
                                    slot + 4,
                                ),
                            )
                        }));
                    }
                }
            }
        }

        impl<C, M> Element<ClientState> for CharacterWrapper<C, M>
        where
            C: Path<ClientState, CharacterSlots>,
            M: Path<ClientState, Option<usize>>,
        {
            type LayoutInfo = Vec<RowLayoutInfo>;

            fn create_layout_info(
                &mut self,
                state: &Context<ClientState>,
                mut store: ElementStoreMut<'_>,
                resolver: &mut Resolver<'_, ClientState>,
            ) -> Self::LayoutInfo {
                self.correct_element_size(state);
                let (_area, layout_info) = resolver.with_derived(10.0, 0.0, |resolver| {
                    self.item_boxes
                        .iter_mut()
                        .enumerate()
                        .map(|(index, item_box)| item_box.create_layout_info(state, store.child_store(index as u64), resolver))
                        .collect()
                });

                layout_info
            }

            fn lay_out<'a>(
                &'a self,
                state: &'a Context<ClientState>,
                store: ElementStore<'a>,
                layout_info: &'a Self::LayoutInfo,
                layout: &mut WindowLayout<'a, ClientState>,
            ) {
                layout.with_layer(|layout| {
                    for (index, item_box) in self.item_boxes.iter().enumerate() {
                        item_box.lay_out(state, store.child_store(index as u64), &layout_info[index], layout);
                    }
                });
            }
        }

        window! {
            title: "Select Character",
            class: Self::window_class(),
            theme: InterfaceThemeType::Menu,
            minimum_width: 900.0,
            maximum_width: 900.0,
            elements: (
                fragment! {
                    gaps: 8.0,
                    children: (
                        CharacterWrapper::new(self.character_slots, self.switch_request),
                        button! {
                            text: client_state().localization().log_out_button_text(),
                            event: InputEvent::LogOutCharacter,
                        },
                    ),
                },
            ),
        }
    }
}
