use derive_new::new;
use korangar_interface::application::FontSizeTrait;
use korangar_interface::element::Element;
use korangar_interface::element::id::ElementIdGenerator;
use korangar_interface::element::store::ElementStore;
use korangar_interface::event::ClickAction;
use korangar_interface::layout::{Layout, Resolver};
use korangar_networking::{InventoryItem, InventoryItemDetails};
use rust_state::{Context, Path};

use crate::graphics::Color;
use crate::input::MouseInputMode;
use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};
use crate::interface::resource::{ItemSource, Move, PartialMove};
use crate::inventory::Skill;
use crate::loaders::{FontSize, Scaling};
use crate::renderer::{InterfaceRenderer, SpriteRenderer};
use crate::state::ClientState;
use crate::world::ResourceMetadata;

pub struct SkillBox<P> {
    item_path: P,
    source: ItemSource,
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
        Self::Layouted { area }
    }

    fn create_layout<'a>(
        &'a self,
        state: &'a Context<ClientState>,
        store: &'a ElementStore,
        layouted: &'a Self::Layouted,
        layout: &mut Layout<'a, ClientState>,
    ) {
    }
}
