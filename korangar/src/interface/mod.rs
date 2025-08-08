pub mod cursor;
pub mod layout;
pub mod resource;
pub mod windows;

pub mod components {
    pub mod item_box {
        use korangar_interface::MouseMode;
        use korangar_interface::element::store::{ElementStore, ElementStoreMut};
        use korangar_interface::element::{BaseLayoutInfo, Element};
        use korangar_interface::event::{ClickAction, Event, EventQueue};
        use korangar_interface::layout::area::Area;
        use korangar_interface::layout::{DropHandler, Layout, MouseButton, Resolver};
        use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
        use korangar_networking::{InventoryItem, InventoryItemDetails};
        use rust_state::{Context, Path};

        use crate::graphics::Color;
        use crate::input::{InputEvent, MouseInputMode};
        use crate::interface::layout::CornerRadius;
        use crate::interface::resource::ItemSource;
        use crate::loaders::{FontSize, OverflowBehavior};
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
            fn handle_drop(&self, _: &Context<ClientState>, queue: &mut EventQueue<ClientState>, mouse_mode: &MouseMode<ClientState>) {
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
            type LayoutInfo = BaseLayoutInfo;

            fn create_layout_info(
                &mut self,
                state: &Context<ClientState>,
                _: ElementStoreMut<'_>,
                resolver: &mut Resolver<'_, ClientState>,
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

            fn lay_out<'a>(
                &'a self,
                state: &'a Context<ClientState>,
                _: ElementStore<'a>,
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
                    let texture_size = layout_info.area.width.min(layout_info.area.height);
                    let texture_area = Area {
                        left: layout_info.area.left + (layout_info.area.width - texture_size) / 2.0,
                        top: layout_info.area.top + (layout_info.area.height - texture_size) / 2.0,
                        width: texture_size,
                        height: texture_size,
                    };

                    layout.add_texture(texture.clone(), texture_area, Color::WHITE, false);

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
                            HorizontalAlignment::Right { offset: 3.0, border: 3.0 },
                            // TODO: Put this in the theme
                            VerticalAlignment::Bottom { offset: 3.0 },
                            OverflowBehavior::Shrink,
                        );
                    }
                }
            }
        }
    }

    pub mod skill_box {
        use korangar_interface::MouseMode;
        use korangar_interface::element::store::{ElementStore, ElementStoreMut};
        use korangar_interface::element::{BaseLayoutInfo, Element};
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
        use crate::loaders::{FontSize, OverflowBehavior};
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
            fn handle_drop(&self, _: &Context<ClientState>, queue: &mut EventQueue<ClientState>, mouse_mode: &MouseMode<ClientState>) {
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
            type LayoutInfo = BaseLayoutInfo;

            fn create_layout_info(
                &mut self,
                state: &Context<ClientState>,
                _: ElementStoreMut<'_>,
                resolver: &mut Resolver<'_, ClientState>,
            ) -> Self::LayoutInfo {
                let area = resolver.with_height(40.0);

                if let Some(skill) = state.try_get(&self.skill_path) {
                    self.level_display.update(skill.skill_level);
                }

                Self::LayoutInfo { area }
            }

            fn lay_out<'a>(
                &'a self,
                state: &'a Context<ClientState>,
                _: ElementStore<'a>,
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
                        HorizontalAlignment::Right { offset: 3.0, border: 3.0 },
                        // TODO: Put this in the theme
                        VerticalAlignment::Bottom { offset: 3.0 },
                        OverflowBehavior::Shrink,
                    );
                }
            }
        }
    }
}
