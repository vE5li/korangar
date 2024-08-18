use std::marker::PhantomData;

use rust_state::Context;

use crate::application::Application;
use crate::elements::{Element2, ElementAllocator, ElementHandle, Focusable, HoverCheck, ModeCheck, Resolve, VTable};

/// Type state [`CloseButton`] builder. This builder utilizes the type system to
/// prevent calling the same method multiple times and calling
/// [`build`](Self::build) before the mandatory methods have been called.
#[derive(Default)]
#[must_use = "`build` needs to be called"]
pub struct CloseButtonBuilder<App>
where
    App: Application,
{
    _marker: PhantomData<App>,
}

impl<App: Application> CloseButtonBuilder<App> {
    pub fn new() -> Self {
        Self { _marker: PhantomData }
    }

    /// Take the builder and turn it into a [`CloseButton`].
    pub fn build(
        self,
        state: &Context<App>,
        allocator: &mut ElementAllocator<App>,
        parent_handle: Option<ElementHandle>,
        theme_selector: App::ThemeSelector,
    ) -> ElementHandle {
        let vtable = const {
            &VTable {
                on_initialize: None,
                on_is_focusable: Focusable::No,
                on_resolve: Resolve::Custom(super::resolve::<App>),
                hover_check: HoverCheck::Default,
                mode_check: ModeCheck::Default,
                on_left_click: Some(super::on_click::<App>),
                on_right_click: None,
                on_drag: None,
                on_input_character: None,
                on_drop_resource: None,
                on_scroll: None,
                background: Some(super::background_color_thing::<App>),
                render: super::render::<App>,
            }
        };

        Element2::new(vtable, None, state, allocator, parent_handle, theme_selector)
    }
}
