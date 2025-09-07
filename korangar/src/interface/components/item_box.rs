use korangar_interface::MouseMode;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{BaseLayoutInfo, Element};
use korangar_interface::event::{ClickHandler, Event, EventQueue};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{DropHandler, MouseButton, Resolver, WindowLayout};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_networking::{InventoryItem, InventoryItemDetails};
use rust_state::{Context, Path};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::{InputEvent, MouseInputMode};
use crate::interface::resource::ItemSource;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::renderer::LayoutExt;
use crate::state::ClientState;
use crate::world::ResourceMetadata;

#[derive(Default)]
struct AmountDisplay {
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

struct ItemBoxHandler<P> {
    item_path: P,
    source: ItemSource,
}

impl<P> ItemBoxHandler<P> {
    fn new(item_path: P, source: ItemSource) -> Self {
        Self { item_path, source }
    }
}

impl<P> ClickHandler<ClientState> for ItemBoxHandler<P>
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

pub struct ItemBox<A> {
    item_path: A,
    handler: ItemBoxHandler<A>,
    amount_display: AmountDisplay,
}

impl<A> ItemBox<A>
where
    A: Copy,
{
    /// This function is supposed to be called from a component macro
    /// and not intended to be called manually.
    #[inline(always)]
    pub fn component_new(item_path: A, source: ItemSource) -> Self {
        Self {
            item_path,
            handler: ItemBoxHandler::new(item_path, source),
            amount_display: AmountDisplay::default(),
        }
    }
}

impl<A> Element<ClientState> for ItemBox<A>
where
    A: Path<ClientState, InventoryItem<ResourceMetadata>, false>,
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
            && let InventoryItemDetails::Regular { amount, .. } = &item.details
        {
            self.amount_display.update(*amount);
        }

        Self::LayoutInfo { area }
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let (is_hovered, background_color) = match layout.get_mouse_mode() {
            MouseMode::Custom {
                mode: MouseInputMode::MoveItem { .. },
            } => match layout_info.area.check().any_mouse_mode().run(layout) {
                true => {
                    // Since we are not in default mouse mode we need to mark the window as
                    // hovered.
                    layout.set_hovered();

                    (true, Color::rgb_u8(80, 180, 180))
                }
                false => (false, Color::rgb_u8(180, 180, 80)),
            },
            _ => match layout_info.area.check().run(layout) {
                true => (true, Color::rgb_u8(60, 60, 60)),
                false => (false, Color::rgb_u8(40, 40, 40)),
            },
        };

        layout.add_rectangle(
            layout_info.area,
            CornerDiameter::uniform(20.0),
            background_color,
            Color::rgba_u8(0, 0, 0, 100),
            ShadowPadding::diagonal(2.0, 5.0),
        );

        if is_hovered {
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

            layout.add_texture(texture_area, texture.clone(), Color::WHITE, false);

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
                    Color::rgb_u8(255, 160, 60),
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
