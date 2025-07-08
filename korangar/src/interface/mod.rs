pub mod layout;
pub mod theme;
#[macro_use]
pub mod elements;
pub mod application;
pub mod cursor;
// pub mod dialog;
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
        use korangar_interface::layout::area::Area;
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

        pub struct CharacterSlotPreview<P, M, A, B> {
            pub character_information: P,
            pub switch_request: M,
            pub background_color: A,
            pub click_handler: CharacterSlotPreviewHandler<B, P>,
            pub slot: usize,
        }

        impl<P, M, A, B> Element<ClientState> for CharacterSlotPreview<P, M, A, B>
        where
            P: Path<ClientState, CharacterInformation, false>,
            M: Path<ClientState, Option<usize>>,
            A: Selector<ClientState, Color>,
            B: Path<ClientState, Option<usize>>,
        {
            fn make_layout(
                &mut self,
                state: &Context<ClientState>,
                store: &mut ElementStore,
                generator: &mut ElementIdGenerator,
                resolver: &mut Resolver,
            ) -> Self::Layouted {
                let area = resolver.with_height(180.0);
                Self::Layouted { area }
            }

            fn create_layout<'a>(
                &'a self,
                state: &'a Context<ClientState>,
                _: &'a ElementStore,
                layouted: &'a Self::Layouted,
                layout: &mut Layout<'a, ClientState>,
            ) {
                if let Some(switch_request) = state.get(&self.switch_request) {
                    let is_hoverered = layout.is_area_hovered_and_active(layouted.area);

                    let background_color = match is_hoverered {
                        true => Color::monochrome_u8(95),
                        false => *state.get(&self.background_color),
                    };
                    layout.add_rectangle(layouted.area, CornerRadius::uniform(25.0), background_color);

                    if *switch_request == self.slot {
                        layout.add_text(
                            layouted.area,
                            "Cancel",
                            FontSize(14.0),
                            Color::WHITE,
                            HorizontalAlignment::Center { offset: 0.0 },
                            VerticalAlignment::Center { offset: 0.0 },
                        );

                        if is_hoverered {
                            layout.add_click_area(layouted.area, &self.click_handler.cancel_switch);
                        }
                    } else {
                        layout.add_text(
                            layouted.area,
                            "Switch slots",
                            FontSize(14.0),
                            Color::WHITE,
                            HorizontalAlignment::Center { offset: 0.0 },
                            VerticalAlignment::Center { offset: 0.0 },
                        );

                        if is_hoverered {
                            layout.add_click_area(layouted.area, &self.click_handler.request_switch);
                        }
                    }

                    return;
                }

                if let Some(character_information) = state.try_get(&self.character_information) {
                    let switch_area = Area {
                        x: layouted.area.x,
                        y: layouted.area.y + layouted.area.height - 40.0,
                        width: 50.0,
                        height: 30.0,
                    };

                    let is_switch_hoverered = layout.is_area_hovered_and_active(switch_area);
                    if is_switch_hoverered {
                        layout.mark_hovered();
                        layout.add_click_area(layouted.area, &self.click_handler.start_switch);
                    }

                    let delete_area = Area {
                        x: layouted.area.x + layouted.area.width - 50.0,
                        y: layouted.area.y + layouted.area.height - 40.0,
                        width: 50.0,
                        height: 30.0,
                    };

                    let is_delete_hoverered = layout.is_area_hovered_and_active(delete_area);
                    if is_delete_hoverered {
                        layout.mark_hovered();
                        layout.add_click_area(layouted.area, &self.click_handler.delete_character);
                    }

                    let is_hoverered = layout.is_area_hovered_and_active(layouted.area);

                    let background_color = match is_hoverered {
                        true => Color::monochrome_u8(95),
                        false => *state.get(&self.background_color),
                    };
                    layout.add_rectangle(layouted.area, CornerRadius::uniform(25.0), background_color);

                    let background_color = match is_switch_hoverered {
                        true => Color::monochrome_u8(180),
                        false => Color::monochrome_u8(150),
                    };
                    layout.add_rectangle(switch_area, CornerRadius::uniform(25.0), background_color);

                    let background_color = match is_delete_hoverered {
                        true => Color::rgb_u8(255, 70, 70),
                        false => Color::rgb_u8(180, 50, 50),
                    };
                    layout.add_rectangle(delete_area, CornerRadius::uniform(25.0), background_color);

                    layout.add_text(
                        layouted.area,
                        &character_information.name,
                        FontSize(18.0),
                        Color::rgb_u8(255, 200, 150),
                        HorizontalAlignment::Left { offset: 5.0 },
                        VerticalAlignment::Top { offset: 0.0 },
                    );

                    layout.add_text(
                        layouted.area,
                        "Base level",
                        FontSize(14.0),
                        Color::rgb_u8(200, 200, 150),
                        HorizontalAlignment::Right { offset: 5.0 },
                        VerticalAlignment::Top { offset: 0.0 },
                    );

                    layout.add_text(
                        layouted.area,
                        self.click_handler
                            .base_level_str
                            .get_str(self.character_information.manually_asserted().base_level(), state),
                        FontSize(14.0),
                        Color::rgb_u8(200, 200, 150),
                        HorizontalAlignment::Right { offset: 5.0 },
                        VerticalAlignment::Top { offset: 14.0 },
                    );

                    layout.add_text(
                        layouted.area,
                        "Job level",
                        FontSize(14.0),
                        Color::rgb_u8(200, 200, 150),
                        HorizontalAlignment::Right { offset: 5.0 },
                        VerticalAlignment::Top { offset: 36.0 },
                    );

                    layout.add_text(
                        layouted.area,
                        self.click_handler
                            .job_level_str
                            .get_str(self.character_information.manually_asserted().job_level(), state),
                        FontSize(14.0),
                        Color::rgb_u8(200, 200, 150),
                        HorizontalAlignment::Right { offset: 5.0 },
                        VerticalAlignment::Top { offset: 50.0 },
                    );

                    layout.add_text(
                        layouted.area,
                        "Map",
                        FontSize(14.0),
                        Color::rgb_u8(200, 200, 150),
                        HorizontalAlignment::Right { offset: 5.0 },
                        VerticalAlignment::Top { offset: 72.0 },
                    );

                    layout.add_text(
                        layouted.area,
                        &character_information.map_name,
                        FontSize(14.0),
                        Color::rgb_u8(200, 200, 150),
                        HorizontalAlignment::Right { offset: 5.0 },
                        VerticalAlignment::Top { offset: 86.0 },
                    );

                    if is_hoverered {
                        layout.add_click_area(layouted.area, &self.click_handler.select_character);
                        layout.mark_hovered();
                    }
                } else {
                    let is_hoverered = layout.is_area_hovered_and_active(layouted.area);

                    let background_color = match is_hoverered {
                        true => Color::monochrome_u8(95),
                        false => *state.get(&self.background_color),
                    };
                    layout.add_rectangle(layouted.area, CornerRadius::uniform(25.0), background_color);

                    layout.add_text(
                        layouted.area,
                        "Create Character",
                        FontSize(14.0),
                        Color::WHITE,
                        HorizontalAlignment::Center { offset: 0.0 },
                        VerticalAlignment::Center { offset: 0.0 },
                    );

                    if is_hoverered {
                        layout.add_click_area(layouted.area, &self.click_handler.create_character);
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
                queue.queue(UserEvent::SelectCharacter { slot: self.slot });
            }
        }

        struct CreateCharacter {
            slot: usize,
        }

        impl ClickAction<ClientState> for CreateCharacter {
            fn execute(&self, _: &Context<ClientState>, queue: &mut EventQueue<ClientState>) {
                queue.queue(UserEvent::OpenCharacterCreationWindow { slot: self.slot });
            }
        }

        struct StartSwitch<P> {
            switch_request: P,
            slot: usize,
        }

        impl<P> ClickAction<ClientState> for StartSwitch<P>
        where
            P: Path<ClientState, Option<usize>>,
        {
            fn execute(&self, state: &Context<ClientState>, _: &mut EventQueue<ClientState>) {
                state.update_value(self.switch_request, Some(self.slot));
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

                queue.queue(UserEvent::SwitchCharacterSlot {
                    origin_slot,
                    destination_slot: self.slot,
                });
            }
        }

        struct DeleteCharacter<P> {
            character_information: P,
        }

        impl<P> ClickAction<ClientState> for DeleteCharacter<P>
        where
            P: Path<ClientState, CharacterInformation, false>,
        {
            fn execute(&self, state: &Context<ClientState>, queue: &mut EventQueue<ClientState>) {
                // SAFETY
                // We should not be able to get here if the character is not present, so it's
                // fine to unwrap.
                let character_information = state.try_get(&self.character_information).unwrap();
                let character_id = character_information.character_id;

                queue.queue(UserEvent::DeleteCharacter { character_id });
            }
        }

        pub struct CharacterSlotPreviewHandler<P, D> {
            select_character: SelectCharacter,
            create_character: CreateCharacter,
            start_switch: StartSwitch<P>,
            cancel_switch: CancelSwitch<P>,
            request_switch: RequestSwitch<P>,
            delete_character: DeleteCharacter<D>,
            base_level_str: PartialEqDisplayStr<i16>,
            job_level_str: PartialEqDisplayStr<i32>,
        }

        impl<P, D> CharacterSlotPreviewHandler<P, D>
        where
            P: Path<ClientState, Option<usize>>,
            D: Path<ClientState, CharacterInformation, false>,
        {
            pub fn new(switch_request: P, character_information: D, slot: usize) -> Self {
                Self {
                    select_character: SelectCharacter { slot },
                    create_character: CreateCharacter { slot },
                    start_switch: StartSwitch { switch_request, slot },
                    cancel_switch: CancelSwitch { switch_request },
                    request_switch: RequestSwitch { switch_request, slot },
                    delete_character: DeleteCharacter { character_information },
                    base_level_str: PartialEqDisplayStr::new(),
                    job_level_str: PartialEqDisplayStr::new(),
                }
            }
        }
    }

    pub mod item_box {
        use korangar_interface::application::FontSizeTrait;
        use korangar_interface::element::Element;
        use korangar_interface::element::id::ElementIdGenerator;
        use korangar_interface::element::store::ElementStore;
        use korangar_interface::event::ClickAction;
        use korangar_interface::layout::{Layout, Resolver};
        use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
        use korangar_networking::{InventoryItem, InventoryItemDetails};
        use rust_state::{Context, Path};

        use crate::graphics::Color;
        use crate::input::MouseInputMode;
        use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};
        use crate::interface::resource::{ItemSource, Move, PartialMove};
        use crate::loaders::{FontSize, Scaling};
        use crate::renderer::{InterfaceRenderer, SpriteRenderer};
        use crate::state::{ClientState, LayoutExt};
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

        pub struct ItemBox<P> {
            pub item_path: P,
            pub source: ItemSource,
            pub amount_display: AmountDisplay,
        }

        impl<P> Element<ClientState> for ItemBox<P>
        where
            P: Path<ClientState, InventoryItem<ResourceMetadata>, false>,
        {
            fn make_layout(
                &mut self,
                state: &Context<ClientState>,
                store: &mut ElementStore,
                generator: &mut ElementIdGenerator,
                resolver: &mut Resolver,
            ) -> Self::Layouted {
                let area = resolver.with_height(40.0);

                if let Some(item) = state.try_get(&self.item_path)
                    && let Some(texture) = item.metadata.texture.as_ref()
                {
                    if let InventoryItemDetails::Regular { amount, .. } = &item.details {
                        self.amount_display.update(*amount);
                    }
                }

                Self::Layouted { area }
            }

            fn create_layout<'a>(
                &'a self,
                state: &'a Context<ClientState>,
                store: &'a ElementStore,
                layouted: &'a Self::Layouted,
                layout: &mut Layout<'a, ClientState>,
            ) {
                layout.add_rectangle(layouted.area, CornerRadius::uniform(20.0), Color::rgb_u8(200, 120, 120));

                if let Some(item) = state.try_get(&self.item_path)
                    && let Some(texture) = item.metadata.texture.as_ref()
                {
                    layout.add_texture(texture.clone(), layouted.area, Color::WHITE, false);

                    if let InventoryItemDetails::Regular { amount, .. } = &item.details {
                        layout.add_text(
                            layouted.area,
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
        use korangar_interface::application::FontSizeTrait;
        use korangar_interface::element::Element;
        use korangar_interface::element::id::ElementIdGenerator;
        use korangar_interface::element::store::ElementStore;
        use korangar_interface::event::ClickAction;
        use korangar_interface::layout::{Layout, Resolver};
        use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
        use korangar_networking::{InventoryItem, InventoryItemDetails};
        use ragnarok_packets::SkillLevel;
        use rust_state::{Context, Path};

        use crate::graphics::Color;
        use crate::input::MouseInputMode;
        use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};
        use crate::interface::resource::{ItemSource, Move, PartialMove, SkillSource};
        use crate::inventory::Skill;
        use crate::loaders::{FontSize, Scaling};
        use crate::renderer::{InterfaceRenderer, SpriteRenderer};
        use crate::state::{ClientState, LayoutExt};
        use crate::world::ResourceMetadata;

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

        pub struct SkillBox<P> {
            pub skill_path: P,
            pub source: SkillSource,
            pub level_display: LevelDisplay,
        }

        impl<P> Element<ClientState> for SkillBox<P>
        where
            P: Path<ClientState, Skill, false>,
        {
            fn make_layout(
                &mut self,
                state: &Context<ClientState>,
                store: &mut ElementStore,
                generator: &mut ElementIdGenerator,
                resolver: &mut Resolver,
            ) -> Self::Layouted {
                let area = resolver.with_height(30.0);

                if let Some(skill) = state.try_get(&self.skill_path) {
                    self.level_display.update(skill.skill_level);
                }

                Self::Layouted { area }
            }

            fn create_layout<'a>(
                &'a self,
                state: &'a Context<ClientState>,
                store: &'a ElementStore,
                layouted: &'a Self::Layouted,
                layout: &mut Layout<'a, ClientState>,
            ) {
                layout.add_rectangle(layouted.area, CornerRadius::uniform(20.0), Color::rgb_u8(200, 120, 120));

                if let Some(skill) = state.try_get(&self.skill_path) {
                    layout.add_sprite(
                        &skill.actions,
                        &skill.sprite,
                        &skill.animation_state,
                        layouted.area,
                        Color::WHITE,
                        false,
                    );

                    layout.add_text(
                        layouted.area,
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
