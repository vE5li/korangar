pub mod cursor;
pub mod layout;
pub mod resource;
pub mod windows;

pub mod components {
    pub mod character_slot_preview {
        use std::cell::UnsafeCell;
        use std::fmt::Display;

        use korangar_interface::element::id::ElementIdGenerator;
        use korangar_interface::element::store::ElementStore;
        use korangar_interface::element::{Element, ErasedElement};
        use korangar_interface::event::{ClickAction, Event, EventQueue};
        use korangar_interface::layout::alignment::{HorizontalAlignment, VerticalAlignment};
        use korangar_interface::layout::tooltip::TooltipExt;
        use korangar_interface::layout::{Layout, MouseButton, Resolver};
        use ragnarok_packets::{CharacterInformation, CharacterInformationPathExt};
        use rust_state::{Context, ManuallyAssertExt, Path, RustState};

        use crate::graphics::Color;
        use crate::input::InputEvent;
        use crate::interface::layout::{CornerRadius, ScreenPosition, ScreenSize};
        use crate::loaders::FontSize;
        use crate::state::ClientState;

        pub struct OverlayHandler<A, B> {
            position: ScreenPosition,
            size: ScreenSize,
            slot: usize,
            switch_request_path: A,
            character_information_path: B,
        }

        impl<A, B> OverlayHandler<A, B> {
            pub fn new(slot: usize, switch_request_path: A, character_information_path: B) -> Self {
                Self {
                    position: ScreenPosition { left: 0.0, top: 0.0 },
                    size: ScreenSize { width: 0.0, height: 0.0 },
                    slot,
                    switch_request_path,
                    character_information_path,
                }
            }

            fn set_position_size(&mut self, position: ScreenPosition, size: ScreenSize) {
                self.position = position;
                self.size = size;
            }
        }

        impl<A, B> ClickAction<ClientState> for OverlayHandler<A, B>
        where
            A: Path<ClientState, Option<usize>>,
            B: Path<ClientState, CharacterInformation, false>,
        {
            fn execute(&self, _: &Context<ClientState>, queue: &mut EventQueue<ClientState>) {
                use korangar_interface::prelude::*;

                let slot = self.slot;
                let switch_request_path = self.switch_request_path;
                let character_information_path = self.character_information_path;

                let erased_element = ErasedElement::new(fragment! {
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
                    element: Box::new(erased_element),
                    position: self.position,
                    size: self.size,
                });
            }
        }

        #[derive(RustState)]
        pub struct CharacterSlotPreviewTheme {
            pub background_color: ClientState,
        }

        pub struct CharacterSlotPreview<P, M, B> {
            pub character_information: P,
            pub switch_request: M,
            pub click_handler: CharacterSlotPreviewHandler<B>,
            pub overlay_handler: OverlayHandler<M, P>,
            pub slot: usize,
        }

        impl<P, M, B> Element<ClientState> for CharacterSlotPreview<P, M, B>
        where
            P: Path<ClientState, CharacterInformation, false>,
            M: Path<ClientState, Option<usize>>,
            B: Path<ClientState, Option<usize>>,
        {
            fn create_layout_info(
                &mut self,
                _: &Context<ClientState>,
                _: &mut ElementStore,
                _: &mut ElementIdGenerator,
                resolver: &mut Resolver,
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
                );

                Self::LayoutInfo { area }
            }

            fn layout_element<'a>(
                &'a self,
                state: &'a Context<ClientState>,
                _: &'a ElementStore,
                layout_info: &'a Self::LayoutInfo,
                layout: &mut Layout<'a, ClientState>,
            ) {
                if let Some(switch_request) = state.get(&self.switch_request) {
                    let is_hoverered = layout.is_area_hovered_and_active(layout_info.area);

                    let background_color = match is_hoverered {
                        true => Color::monochrome_u8(80),
                        false => Color::monochrome_u8(60),
                    };
                    layout.add_rectangle(layout_info.area, CornerRadius::uniform(25.0), background_color);

                    if *switch_request == self.slot {
                        layout.add_text(
                            layout_info.area,
                            "Cancel",
                            FontSize(14.0),
                            Color::WHITE,
                            HorizontalAlignment::Center { offset: 0.0 },
                            VerticalAlignment::Center { offset: 0.0 },
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
                            HorizontalAlignment::Center { offset: 0.0 },
                            VerticalAlignment::Center { offset: 0.0 },
                        );

                        if is_hoverered {
                            layout.add_click_area(layout_info.area, MouseButton::Left, &self.click_handler.request_switch);
                        }
                    }

                    return;
                }

                if let Some(character_information) = state.try_get(&self.character_information) {
                    let is_hoverered = layout.is_area_hovered_and_active(layout_info.area);

                    let background_color = match is_hoverered {
                        true => Color::monochrome_u8(110),
                        false => Color::monochrome_u8(90),
                    };
                    layout.add_rectangle(layout_info.area, CornerRadius::uniform(25.0), background_color);

                    layout.add_text(
                        layout_info.area,
                        &character_information.name,
                        FontSize(18.0),
                        Color::rgb_u8(255, 200, 150),
                        HorizontalAlignment::Center { offset: 0.0 },
                        VerticalAlignment::Top { offset: 0.0 },
                    );

                    layout.add_text(
                        layout_info.area,
                        "Base level",
                        FontSize(14.0),
                        Color::rgb_u8(200, 200, 150),
                        HorizontalAlignment::Left { offset: 5.0 },
                        VerticalAlignment::Top { offset: 30.0 },
                    );

                    layout.add_text(
                        layout_info.area,
                        self.click_handler
                            .base_level_str
                            .get_str(self.character_information.manually_asserted().base_level(), state),
                        FontSize(14.0),
                        Color::rgb_u8(200, 200, 150),
                        HorizontalAlignment::Left { offset: 5.0 },
                        VerticalAlignment::Top { offset: 44.0 },
                    );

                    layout.add_text(
                        layout_info.area,
                        "Job level",
                        FontSize(14.0),
                        Color::rgb_u8(200, 200, 150),
                        HorizontalAlignment::Left { offset: 5.0 },
                        VerticalAlignment::Top { offset: 66.0 },
                    );

                    layout.add_text(
                        layout_info.area,
                        self.click_handler
                            .job_level_str
                            .get_str(self.character_information.manually_asserted().job_level(), state),
                        FontSize(14.0),
                        Color::rgb_u8(200, 200, 150),
                        HorizontalAlignment::Left { offset: 5.0 },
                        VerticalAlignment::Top { offset: 80.0 },
                    );

                    layout.add_text(
                        layout_info.area,
                        "Map",
                        FontSize(14.0),
                        Color::rgb_u8(200, 200, 150),
                        HorizontalAlignment::Left { offset: 5.0 },
                        VerticalAlignment::Top { offset: 102.0 },
                    );

                    layout.add_text(
                        layout_info.area,
                        &character_information.map_name,
                        FontSize(14.0),
                        Color::rgb_u8(200, 200, 150),
                        HorizontalAlignment::Left { offset: 5.0 },
                        VerticalAlignment::Top { offset: 116.0 },
                    );

                    if is_hoverered {
                        layout.add_click_area(layout_info.area, MouseButton::Left, &self.click_handler.select_character);
                        layout.add_click_area(layout_info.area, MouseButton::Right, &self.overlay_handler);
                        layout.mark_hovered();

                        {
                            struct PrivateTooltipId;
                            layout.add_tooltip(&character_information.name, PrivateTooltipId.tooltip_id());
                        }
                    }
                } else {
                    let is_hoverered = layout.is_area_hovered_and_active(layout_info.area);

                    let background_color = match is_hoverered {
                        true => Color::monochrome_u8(55),
                        false => Color::monochrome_u8(40),
                    };
                    layout.add_rectangle(layout_info.area, CornerRadius::uniform(25.0), background_color);

                    layout.add_text(
                        layout_info.area,
                        "Create Character",
                        FontSize(14.0),
                        Color::monochrome_u8(85),
                        HorizontalAlignment::Center { offset: 0.0 },
                        VerticalAlignment::Center { offset: 0.0 },
                    );

                    if is_hoverered {
                        layout.add_click_area(layout_info.area, MouseButton::Left, &self.click_handler.create_character);
                        layout.mark_hovered();
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

        impl ClickAction<ClientState> for SelectCharacter {
            fn execute(&self, _: &Context<ClientState>, queue: &mut EventQueue<ClientState>) {
                queue.queue(InputEvent::SelectCharacter { slot: self.slot });
            }
        }

        struct CreateCharacter {
            slot: usize,
        }

        impl ClickAction<ClientState> for CreateCharacter {
            fn execute(&self, _: &Context<ClientState>, queue: &mut EventQueue<ClientState>) {
                queue.queue(InputEvent::OpenCharacterCreationWindow { slot: self.slot });
            }
        }

        struct CancelSwitch<P> {
            switch_request: P,
        }

        impl<P> ClickAction<ClientState> for CancelSwitch<P>
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

        impl<P> ClickAction<ClientState> for RequestSwitch<P>
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

    pub mod item_box {
        use korangar_interface::MouseMode;
        use korangar_interface::element::Element;
        use korangar_interface::element::id::ElementIdGenerator;
        use korangar_interface::element::store::ElementStore;
        use korangar_interface::event::{ClickAction, Event, EventQueue};
        use korangar_interface::layout::{DropHandler, Layout, MouseButton, Resolver};
        use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
        use korangar_networking::{InventoryItem, InventoryItemDetails};
        use rust_state::{Context, Path};

        use crate::graphics::Color;
        use crate::input::{InputEvent, MouseInputMode};
        use crate::interface::layout::CornerRadius;
        use crate::interface::resource::ItemSource;
        use crate::loaders::FontSize;
        use crate::renderer::LayoutExt;
        use crate::state::ClientState;
        use crate::world::ResourceMetadata;

        #[derive(Default)]
        pub struct AmountDisplay {
            amount: u16,
            string: Option<String>,
        }

        impl AmountDisplay {
            fn update(&mut self, new_amount: u16) {
                if self.string.is_none() || self.amount != new_amount {
                    self.string = Some(new_amount.to_string());
                    self.amount = new_amount;
                }
            }
        }

        pub struct ItemBoxHandler<P> {
            item_path: P,
            source: ItemSource,
        }

        impl<P> ItemBoxHandler<P> {
            pub fn new(item_path: P, source: ItemSource) -> Self {
                Self { item_path, source }
            }
        }

        impl<P> ClickAction<ClientState> for ItemBoxHandler<P>
        where
            P: Path<ClientState, InventoryItem<ResourceMetadata>, false>,
        {
            fn execute(&self, state: &Context<ClientState>, queue: &mut EventQueue<ClientState>) {
                // SAFETY:
                //
                // Unwrapping here is fine since we only register the handler if the slot has a
                // item.
                let item = state.try_get(&self.item_path).unwrap().clone();

                queue.queue(Event::SetMouseMode {
                    mouse_mode: MouseMode::Custom {
                        mode: MouseInputMode::MoveItem { item, source: self.source },
                    },
                });
            }
        }

        impl<P> DropHandler<ClientState> for ItemBoxHandler<P>
        where
            P: Path<ClientState, InventoryItem<ResourceMetadata>, false>,
        {
            fn handle_drop(&self, state: &Context<ClientState>, queue: &mut EventQueue<ClientState>, mouse_mode: &MouseMode<ClientState>) {
                if let MouseMode::Custom {
                    mode: MouseInputMode::MoveItem { source, item },
                } = mouse_mode
                {
                    queue.queue(InputEvent::MoveItem {
                        source: *source,
                        destination: self.source,
                        item: item.clone(),
                    });
                }
            }
        }

        pub struct ItemBox<P> {
            pub item_path: P,
            pub handler: ItemBoxHandler<P>,
            pub amount_display: AmountDisplay,
        }

        impl<P> Element<ClientState> for ItemBox<P>
        where
            P: Path<ClientState, InventoryItem<ResourceMetadata>, false>,
        {
            fn create_layout_info(
                &mut self,
                state: &Context<ClientState>,
                _: &mut ElementStore,
                _: &mut ElementIdGenerator,
                resolver: &mut Resolver,
            ) -> Self::LayoutInfo {
                let area = resolver.with_height(40.0);

                if let Some(item) = state.try_get(&self.item_path)
                    && item.metadata.texture.as_ref().is_some()
                {
                    if let InventoryItemDetails::Regular { amount, .. } = &item.details {
                        self.amount_display.update(*amount);
                    }
                }

                Self::LayoutInfo { area }
            }

            fn layout_element<'a>(
                &'a self,
                state: &'a Context<ClientState>,
                _: &'a ElementStore,
                layout_info: &'a Self::LayoutInfo,
                layout: &mut Layout<'a, ClientState>,
            ) {
                let (is_hovered, background_color) = match layout.get_mouse_mode() {
                    MouseMode::Custom {
                        mode: MouseInputMode::MoveItem { .. },
                    } => match layout.is_area_hovered_and_active_any_mode(layout_info.area) {
                        true => (true, Color::rgb_u8(80, 180, 180)),
                        false => (false, Color::rgb_u8(180, 180, 80)),
                    },
                    _ => match layout.is_area_hovered_and_active(layout_info.area) {
                        true => (true, Color::rgb_u8(60, 60, 60)),
                        false => (false, Color::rgb_u8(40, 40, 40)),
                    },
                };

                layout.add_rectangle(layout_info.area, CornerRadius::uniform(20.0), background_color);

                if is_hovered {
                    layout.mark_hovered();
                    layout.add_drop_area(layout_info.area, &self.handler);
                }

                if let Some(item) = state.try_get(&self.item_path)
                    && let Some(texture) = item.metadata.texture.as_ref()
                {
                    layout.add_texture(texture.clone(), layout_info.area, Color::WHITE, false);

                    if is_hovered {
                        layout.add_click_area(layout_info.area, MouseButton::Left, &self.handler);
                    }

                    if matches!(item.details, InventoryItemDetails::Regular { .. }) {
                        layout.add_text(
                            layout_info.area,
                            self.amount_display.string.as_ref().unwrap(),
                            // TODO: Put this in the theme
                            FontSize(12.0),
                            // TODO: Put this in the theme
                            Color::rgb_u8(255, 200, 255),
                            // TODO: Put this in the theme
                            HorizontalAlignment::Right { offset: 3.0 },
                            // TODO: Put this in the theme
                            VerticalAlignment::Bottom { offset: 3.0 },
                        );
                    }
                }
            }
        }
    }

    pub mod skill_box {
        use korangar_interface::MouseMode;
        use korangar_interface::element::Element;
        use korangar_interface::element::id::ElementIdGenerator;
        use korangar_interface::element::store::ElementStore;
        use korangar_interface::event::{ClickAction, Event, EventQueue};
        use korangar_interface::layout::{DropHandler, Layout, MouseButton, Resolver};
        use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
        use ragnarok_packets::SkillLevel;
        use rust_state::{Context, Path};

        use crate::graphics::Color;
        use crate::input::{InputEvent, MouseInputMode};
        use crate::interface::layout::CornerRadius;
        use crate::interface::resource::SkillSource;
        use crate::inventory::Skill;
        use crate::loaders::FontSize;
        use crate::renderer::LayoutExt;
        use crate::state::ClientState;

        pub struct LevelDisplay {
            level: SkillLevel,
            string: Option<String>,
        }

        impl Default for LevelDisplay {
            fn default() -> Self {
                Self {
                    level: SkillLevel(0),
                    string: Default::default(),
                }
            }
        }

        impl LevelDisplay {
            fn update(&mut self, new_level: SkillLevel) {
                if self.string.is_none() || self.level != new_level {
                    self.string = Some(new_level.0.to_string());
                    self.level = new_level;
                }
            }
        }

        pub struct SkillBoxHandler<P> {
            skill_path: P,
            source: SkillSource,
        }

        impl<P> SkillBoxHandler<P> {
            pub fn new(skill_path: P, source: SkillSource) -> Self {
                Self { skill_path, source }
            }
        }

        impl<P> ClickAction<ClientState> for SkillBoxHandler<P>
        where
            P: Path<ClientState, Skill, false>,
        {
            fn execute(&self, state: &Context<ClientState>, queue: &mut EventQueue<ClientState>) {
                // SAFETY:
                //
                // Unwrapping here is fine since we only register the handler if the slot has a
                // skill.
                let skill = state.try_get(&self.skill_path).unwrap().clone();

                queue.queue(Event::SetMouseMode {
                    mouse_mode: MouseMode::Custom {
                        mode: MouseInputMode::MoveSkill {
                            skill,
                            source: self.source,
                        },
                    },
                });
            }
        }

        impl<P> DropHandler<ClientState> for SkillBoxHandler<P>
        where
            P: Path<ClientState, Skill, false>,
        {
            fn handle_drop(&self, state: &Context<ClientState>, queue: &mut EventQueue<ClientState>, mouse_mode: &MouseMode<ClientState>) {
                if let MouseMode::Custom {
                    mode: MouseInputMode::MoveSkill { source, skill },
                } = mouse_mode
                {
                    queue.queue(InputEvent::MoveSkill {
                        source: *source,
                        destination: self.source,
                        skill: skill.clone(),
                    });
                }
            }
        }

        pub struct SkillBox<P> {
            pub skill_path: P,
            pub handler: SkillBoxHandler<P>,
            pub level_display: LevelDisplay,
        }

        impl<P> Element<ClientState> for SkillBox<P>
        where
            P: Path<ClientState, Skill, false>,
        {
            fn create_layout_info(
                &mut self,
                state: &Context<ClientState>,
                _: &mut ElementStore,
                _: &mut ElementIdGenerator,
                resolver: &mut Resolver,
            ) -> Self::LayoutInfo {
                let area = resolver.with_height(40.0);

                if let Some(skill) = state.try_get(&self.skill_path) {
                    self.level_display.update(skill.skill_level);
                }

                Self::LayoutInfo { area }
            }

            fn layout_element<'a>(
                &'a self,
                state: &'a Context<ClientState>,
                _: &'a ElementStore,
                layout_info: &'a Self::LayoutInfo,
                layout: &mut Layout<'a, ClientState>,
            ) {
                let (is_hovered, background_color) = match layout.get_mouse_mode() {
                    MouseMode::Custom {
                        mode: MouseInputMode::MoveSkill { .. },
                    } => match layout.is_area_hovered_and_active_any_mode(layout_info.area) {
                        true => (true, Color::rgb_u8(80, 180, 180)),
                        false => (false, Color::rgb_u8(180, 180, 80)),
                    },
                    _ => match layout.is_area_hovered_and_active(layout_info.area) {
                        true => (true, Color::rgb_u8(60, 60, 60)),
                        false => (false, Color::rgb_u8(40, 40, 40)),
                    },
                };

                layout.add_rectangle(layout_info.area, CornerRadius::uniform(20.0), background_color);

                if is_hovered {
                    layout.mark_hovered();
                    layout.add_drop_area(layout_info.area, &self.handler);
                }

                if let Some(skill) = state.try_get(&self.skill_path) {
                    layout.add_sprite(
                        &skill.actions,
                        &skill.sprite,
                        &skill.animation_state,
                        layout_info.area,
                        Color::WHITE,
                    );

                    if is_hovered {
                        layout.add_click_area(layout_info.area, MouseButton::Left, &self.handler);
                    }

                    layout.add_text(
                        layout_info.area,
                        self.level_display.string.as_ref().unwrap(),
                        // TODO: Put this in the theme
                        FontSize(12.0),
                        // TODO: Put this in the theme
                        Color::rgb_u8(255, 200, 255),
                        // TODO: Put this in the theme
                        HorizontalAlignment::Right { offset: 3.0 },
                        // TODO: Put this in the theme
                        VerticalAlignment::Bottom { offset: 3.0 },
                    );
                }
            }
        }
    }
}
