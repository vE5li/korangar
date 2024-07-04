use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use rust_state::{Context, Tracker};

use crate::application::{
    Application, ClipTrait, CornerRadiusTraitExt, FontSizeTraitExt, InterfaceRenderer, PartialSizeTraitExt, PositionTrait,
    PositionTraitExt, SizeTrait, SizeTraitExt,
};
use crate::event::{ChangeEvent, ClickAction, HoverInformation};
use crate::layout::{PlacementResolver, SizeBound};

pub type ElementCell<App> = Rc<RefCell<dyn Element<App>>>;
pub type WeakElementCell<App> = Weak<RefCell<dyn Element<App>>>;

pub trait ElementWrap<App>
where
    App: Application,
{
    fn wrap(self) -> ElementCell<App>;
}

impl<App, T> ElementWrap<App> for T
where
    App: Application,
    T: Element<App> + Sized + 'static,
{
    fn wrap(self) -> ElementCell<App> {
        Rc::new(RefCell::new(self))
    }
}

pub struct ElementRenderer<'a, 'b, App>
where
    App: Application,
{
    pub render_target: &'a mut <App::Renderer as InterfaceRenderer<App>>::Target,
    pub renderer: &'a App::Renderer,
    pub state: &'a Tracker<'b, App>,
    pub position: App::Position,
    pub size: App::Size,
    pub clip: App::Clip,
}

impl<'a, 'b, App> ElementRenderer<'a, 'b, App>
where
    App: Application,
{
    pub fn get_position(&self) -> App::Position {
        self.position
    }

    pub fn get_text_dimensions(&self, text: &str, font_size: App::FontSize, available_width: f32) -> App::Size {
        self.renderer.get_text_dimensions(
            text,
            font_size.scaled(*self.state.get_safe(&App::ScaleSelector::default())),
            available_width,
        )
    }

    pub fn set_scroll(&mut self, scroll: f32) {
        self.position = App::Position::new(self.position.left(), self.position.top() - scroll);
    }

    pub fn render_background(&mut self, corner_radius: App::CornerRadius, color: App::Color) {
        self.renderer.render_rectangle(
            self.render_target,
            self.position,
            self.size,
            self.clip,
            corner_radius.scaled(*self.state.get_safe(&App::ScaleSelector::default())),
            color,
        );
    }

    pub fn render_rectangle(&mut self, position: App::Position, size: App::Size, corner_radius: App::CornerRadius, color: App::Color) {
        self.renderer.render_rectangle(
            self.render_target,
            self.position.combined(position),
            size,
            self.clip,
            corner_radius.scaled(*self.state.get_safe(&App::ScaleSelector::default())),
            color,
        );
    }

    pub fn render_text(&mut self, text: &str, offset: App::Position, foreground_color: App::Color, font_size: App::FontSize) -> f32 {
        let scale = *self.state.get_safe(&App::ScaleSelector::default());

        self.renderer.render_text(
            self.render_target,
            text,
            self.position.combined(offset.scaled(scale)),
            self.clip,
            foreground_color,
            font_size.scaled(scale),
        )
    }

    pub fn render_checkbox(&mut self, offset: App::Position, size: App::Size, color: App::Color, checked: bool) {
        let scale = *self.state.get_safe(&App::ScaleSelector::default());

        self.renderer.render_checkbox(
            self.render_target,
            self.position.combined(offset.scaled(scale)),
            size.scaled(scale),
            self.clip,
            color,
            checked,
        );
    }

    pub fn render_expand_arrow(&mut self, offset: App::Position, size: App::Size, color: App::Color, expanded: bool) {
        let scale = *self.state.get_safe(&App::ScaleSelector::default());

        self.renderer.render_expand_arrow(
            self.render_target,
            self.position.combined(offset.scaled(scale)),
            size.scaled(scale),
            self.clip,
            color,
            expanded,
        );
    }

    pub fn render_element(
        &mut self,
        element: &dyn Element<App>,
        application: &Tracker<App>,
        theme_selector: App::ThemeSelector,
        second_theme: bool,
    ) {
        element.render(
            self.render_target,
            self.renderer,
            application,
            theme_selector,
            self.position,
            self.clip,
            second_theme,
        )
    }
}

pub struct ElementState<App>
where
    App: Application,
{
    pub cached_size: App::Size,
    pub cached_position: App::Position,
    pub self_element: Option<WeakElementCell<App>>,
    pub parent_element: Option<WeakElementCell<App>>,
    pub mouse_position: Cell<App::Position>,
}

impl<App> Default for ElementState<App>
where
    App: Application,
{
    fn default() -> Self {
        Self {
            cached_size: App::Size::zero(),
            cached_position: App::Position::zero(),
            self_element: None,
            parent_element: None,
            mouse_position: Cell::new(App::Position::zero()),
        }
    }
}

impl<App> ElementState<App>
where
    App: Application,
{
    pub fn link_back(&mut self, weak_self: WeakElementCell<App>, weak_parent: Option<WeakElementCell<App>>) {
        self.self_element = Some(weak_self);
        self.parent_element = weak_parent;
    }

    pub fn resolve(&mut self, placement_resolver: &mut PlacementResolver<App>, size_bound: &SizeBound) {
        let (size, position) = placement_resolver.allocate(size_bound);
        self.cached_size = size.finalize();
        self.cached_position = position;
    }

    pub fn hovered_element(&self, mouse_position: App::Position) -> HoverInformation<App> {
        let absolute_position = mouse_position.relative_to(self.cached_position);

        if absolute_position.left() >= 0.0
            && absolute_position.top() >= 0.0
            && absolute_position.left() <= self.cached_size.width()
            && absolute_position.top() <= self.cached_size.height()
        {
            self.mouse_position.replace(absolute_position);
            return HoverInformation::Hovered;
        }

        HoverInformation::Missed
    }

    pub fn element_renderer<'a, 'b>(
        &self,
        render_target: &'a mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &'a App::Renderer,
        state: &'a Tracker<'b, App>,
        parent_position: App::Position,
        screen_clip: App::Clip,
    ) -> ElementRenderer<'a, 'b, App> {
        let position = parent_position.combined(self.cached_position);
        let size = self.cached_size;

        let screen_clip = App::Clip::new(
            screen_clip.left().max(position.left()),
            screen_clip.top().max(position.top()),
            screen_clip.right().min(position.left() + self.cached_size.width()),
            screen_clip.bottom().min(position.top() + self.cached_size.height()),
        );

        ElementRenderer {
            render_target,
            renderer,
            state,
            position,
            size,
            clip: screen_clip,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Focus {
    pub mode: FocusMode,
    pub downwards: bool,
}

impl Focus {
    pub fn new(mode: FocusMode) -> Self {
        Self { mode, downwards: false }
    }

    pub fn downwards() -> Self {
        Self {
            mode: FocusMode::FocusNext,
            downwards: true,
        }
    }

    pub fn to_downwards(self) -> Self {
        Focus {
            mode: self.mode,
            downwards: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FocusMode {
    FocusNext,
    FocusPrevious,
}

impl From<bool> for FocusMode {
    fn from(reverse: bool) -> Self {
        match reverse {
            true => Self::FocusPrevious,
            false => Self::FocusNext,
        }
    }
}

pub trait Element<App>
where
    App: Application,
{
    fn get_state(&self) -> &ElementState<App>;

    fn get_state_mut(&mut self) -> &mut ElementState<App>;

    fn link_back(&mut self, weak_self: WeakElementCell<App>, weak_parent: Option<WeakElementCell<App>>) {
        self.get_state_mut().link_back(weak_self, weak_parent);
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn focus_next(&self, self_cell: ElementCell<App>, _caller_cell: Option<ElementCell<App>>, focus: Focus) -> Option<ElementCell<App>> {
        if focus.downwards {
            return Some(self_cell);
        }

        self.get_state().parent_element.as_ref().and_then(|parent_element| {
            let parent_element = parent_element.upgrade().unwrap();
            let next_element = parent_element.borrow().focus_next(parent_element.clone(), Some(self_cell), focus);
            next_element
        })
    }

    fn restore_focus(&self, self_cell: ElementCell<App>) -> Option<ElementCell<App>> {
        self.is_focusable().then_some(self_cell)
    }

    fn resolve(&mut self, application: &Tracker<App>, theme_selector: App::ThemeSelector, placement_resolver: &mut PlacementResolver<App>);

    fn update(&mut self) -> Option<ChangeEvent> {
        None
    }

    fn is_cell_self(&self, element: &Option<ElementCell<App>>) -> bool {
        let element = element.as_ref().map(|element| unsafe { &*element.as_ptr() });
        matches!(element, Some(reference) if std::ptr::eq(reference as *const _ as *const (), self as *const _ as *const ()))
    }

    fn is_element_self(&self, element: &Option<&dyn Element<App>>) -> bool {
        matches!(element, Some(reference) if std::ptr::eq(reference as *const _ as *const (), self as *const _ as *const ()))
    }

    fn hovered_element(&self, _mouse_position: App::Position, _mouse_mode: &App::MouseInputMode) -> HoverInformation<App> {
        HoverInformation::Missed
    }

    fn left_click(&mut self, state: &Context<App>, update: &mut bool) -> Vec<ClickAction<App>> {
        let _ = (state, update);
        Vec::new()
    }

    fn right_click(&mut self, state: &Context<App>, update: &mut bool) -> Vec<ClickAction<App>> {
        let _ = (state, update);
        Vec::new()
    }

    fn drag(&mut self, _mouse_delta: App::Position) -> Option<ChangeEvent> {
        None
    }

    fn input_character(&mut self, state: &Context<App>, character: char) -> (bool, Vec<ClickAction<App>>) {
        let _ = (state, character);
        (false, Vec::new())
    }

    fn drop_resource(&mut self, drop_resource: App::DropResource) -> Option<App::DropResult> {
        let _ = drop_resource;
        None
    }

    fn scroll(&mut self, delta: f32) -> Option<ChangeEvent> {
        self.get_state()
            .parent_element
            .as_ref()
            .and_then(|weak_pointer| weak_pointer.upgrade())
            .and_then(|element| (*element).borrow_mut().scroll(delta))
    }

    fn render(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &App::Renderer,
        application: &Tracker<App>,
        theme_selector: App::ThemeSelector,
        parent_position: App::Position,
        screen_clip: App::Clip,
        second_theme: bool,
    );
}
