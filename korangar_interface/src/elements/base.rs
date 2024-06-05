use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use interface_procedural::{FucusableDefault, LinkBackDefault};

use super::ContainerState;
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

pub struct ElementRenderer<'a, App>
where
    App: Application,
{
    pub render_target: &'a mut <App::Renderer as InterfaceRenderer<App>>::Target,
    pub renderer: &'a App::Renderer,
    pub application: &'a App,
    pub position: App::Position,
    pub size: App::Size,
    pub clip: App::Clip,
}

impl<'a, App> ElementRenderer<'a, App>
where
    App: Application,
{
    pub fn get_position(&self) -> App::Position {
        self.position
    }

    pub fn get_text_dimensions(&self, text: &str, font_size: App::FontSize, available_width: f32) -> App::Size {
        self.renderer
            .get_text_dimensions(text, font_size.scaled(self.application.get_scaling()), available_width)
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
            corner_radius.scaled(self.application.get_scaling()),
            color,
        );
    }

    pub fn render_rectangle(&mut self, position: App::Position, size: App::Size, corner_radius: App::CornerRadius, color: App::Color) {
        self.renderer.render_rectangle(
            self.render_target,
            self.position.combined(position),
            size,
            self.clip,
            corner_radius.scaled(self.application.get_scaling()),
            color,
        );
    }

    pub fn render_text(&mut self, text: &str, offset: App::Position, foreground_color: App::Color, font_size: App::FontSize) -> f32 {
        self.renderer.render_text(
            self.render_target,
            text,
            self.position.combined(offset.scaled(self.application.get_scaling())),
            self.clip,
            foreground_color,
            font_size.scaled(self.application.get_scaling()),
        )
    }

    pub fn render_checkbox(&mut self, offset: App::Position, size: App::Size, color: App::Color, checked: bool) {
        self.renderer.render_checkbox(
            self.render_target,
            self.position.combined(offset.scaled(self.application.get_scaling())),
            size.scaled(self.application.get_scaling()),
            self.clip,
            color,
            checked,
        );
    }

    pub fn render_expand_arrow(&mut self, offset: App::Position, size: App::Size, color: App::Color, expanded: bool) {
        self.renderer.render_expand_arrow(
            self.render_target,
            self.position.combined(offset.scaled(self.application.get_scaling())),
            size.scaled(self.application.get_scaling()),
            self.clip,
            color,
            expanded,
        );
    }

    pub fn render_element(
        &mut self,
        element: &dyn Element<App>,
        application: &App,
        theme: &App::Theme,
        hovered_element: Option<&dyn Element<App>>,
        focused_element: Option<&dyn Element<App>>,
        mouse_mode: &App::MouseInputMode,
        second_theme: bool,
    ) {
        element.render(
            self.render_target,
            self.renderer,
            application,
            theme,
            self.position,
            self.clip,
            hovered_element,
            focused_element,
            mouse_mode,
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

    pub fn element_renderer<'a>(
        &self,
        render_target: &'a mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &'a App::Renderer,
        application: &'a App,
        parent_position: App::Position,
        screen_clip: App::Clip,
    ) -> ElementRenderer<'a, App> {
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
            application,
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

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver<App>, application: &App, theme: &App::Theme);

    fn update(&mut self) -> Option<ChangeEvent> {
        None
    }

    fn is_element_self(&self, element: Option<&dyn Element<App>>) -> bool {
        matches!(element, Some(reference) if std::ptr::eq(reference as *const _ as *const (), self as *const _ as *const ()))
    }

    fn hovered_element(&self, _mouse_position: App::Position, _mouse_mode: &App::MouseInputMode) -> HoverInformation<App> {
        HoverInformation::Missed
    }

    fn left_click(&mut self, _update: &mut bool) -> Vec<ClickAction<App>> {
        Vec::new()
    }

    fn right_click(&mut self, _update: &mut bool) -> Vec<ClickAction<App>> {
        Vec::new()
    }

    fn drag(&mut self, _mouse_delta: App::Position) -> Option<ChangeEvent> {
        None
    }

    fn input_character(&mut self, _character: char) -> Vec<ClickAction<App>> {
        Vec::new()
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
        render: &App::Renderer,
        application: &App,
        theme: &App::Theme,
        parent_position: App::Position,
        screen_clip: App::Clip,
        hovered_element: Option<&dyn Element<App>>,
        focused_element: Option<&dyn Element<App>>,
        mouse_mode: &App::MouseInputMode,
        second_theme: bool,
    );
}

////////////////////////

pub trait StateType<App>
where
    App: Application,
{
    fn link_back(&mut self, weak_self: WeakElementCell<App>, weak_parent: Option<WeakElementCell<App>>);

    fn is_focusable(&self, self_focusable: bool) -> bool;

    fn focus_next(
        &self,
        self_cell: ElementCell<App>,
        caller_cell: Option<ElementCell<App>>,
        focus: Focus,
        self_focusable: bool,
    ) -> Option<ElementCell<App>>;

    fn restore_focus(&self, self_cell: ElementCell<App>, self_focusable: bool) -> Option<ElementCell<App>>;

    fn render(
        &self,
        renderer: &mut ElementRenderer<App>,
        theme: &App::Theme,
        hovered_element: Option<&dyn Element<App>>,
        focused_element: Option<&dyn Element<App>>,
        mouse_mode: &App::MouseInputMode,
        second_theme: bool,
    );

    fn element_renderer<'a>(
        &self,
        render_target: &'a mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &'a App::Renderer,
        application: &'a App,
        parent_position: App::Position,
        screen_clip: App::Clip,
    ) -> ElementRenderer<'a, App>;

    fn hovered_element(&self, mouse_position: App::Position, mouse_mode: &App::MouseInputMode) -> HoverInformation<App>;
}

impl<App> StateType<App> for ElementState<App>
where
    App: Application,
{
    fn link_back(&mut self, weak_self: WeakElementCell<App>, weak_parent: Option<WeakElementCell<App>>) {
        todo!()
    }

    fn is_focusable(&self, self_focusable: bool) -> bool {
        self_focusable
    }

    fn focus_next(
        &self,
        self_cell: ElementCell<App>,
        _caller_cell: Option<ElementCell<App>>,
        focus: Focus,
        self_focusable: bool,
    ) -> Option<ElementCell<App>> {
        // FIX: Shouldn't this only work if focusable is true??
        if focus.downwards {
            return Some(self_cell);
        }

        self.parent_element.as_ref().and_then(|parent_element| {
            let parent_element = parent_element.upgrade().unwrap();
            let next_element = parent_element.borrow().focus_next(parent_element.clone(), Some(self_cell), focus);
            next_element
        })
    }

    fn restore_focus(&self, self_cell: ElementCell<App>, self_focusable: bool) -> Option<ElementCell<App>> {
        self_focusable.then_some(self_cell)
    }

    fn render(
        &self,
        _renderer: &mut ElementRenderer<App>,
        _theme: &App::Theme,
        _hovered_element: Option<&dyn Element<App>>,
        _focused_element: Option<&dyn Element<App>>,
        _mouse_mode: &App::MouseInputMode,
        _second_theme: bool,
    ) {
    }

    fn element_renderer<'a>(
        &self,
        render_target: &'a mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &'a App::Renderer,
        application: &'a App,
        parent_position: App::Position,
        screen_clip: App::Clip,
    ) -> ElementRenderer<'a, App> {
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
            application,
            position,
            size,
            clip: screen_clip,
        }
    }

    fn hovered_element(&self, mouse_position: App::Position, mouse_mode: &App::MouseInputMode) -> HoverInformation<App> {
        todo!()
    }
}

impl<App> StateType<App> for ContainerState<App>
where
    App: Application,
{
    fn link_back(&mut self, weak_self: WeakElementCell<App>, weak_parent: Option<WeakElementCell<App>>) {
        todo!() // Code from ContainerState
    }

    fn is_focusable(&self, self_focusable: bool) -> bool {
        todo!() // Code from ContainerState
    }

    fn focus_next(
        &self,
        self_cell: ElementCell<App>,
        caller_cell: Option<ElementCell<App>>,
        focus: Focus,
        self_focusable: bool,
    ) -> Option<ElementCell<App>> {
        todo!() // Code from ContainerState
    }

    fn restore_focus(&self, self_cell: ElementCell<App>, self_focusable: bool) -> Option<ElementCell<App>> {
        todo!() // Code from ContainerState
    }

    fn render(
        &self,
        renderer: &mut ElementRenderer<App>,
        theme: &App::Theme,
        hovered_element: Option<&dyn Element<App>>,
        focused_element: Option<&dyn Element<App>>,
        mouse_mode: &App::MouseInputMode,
        second_theme: bool,
    ) {
        self.elements.iter().for_each(|element| {
            renderer.render_element(
                &*element.borrow(),
                &renderer.application,
                theme,
                hovered_element,
                focused_element,
                mouse_mode,
                second_theme,
            )
        });
    }

    fn element_renderer<'a>(
        &self,
        render_target: &'a mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &'a App::Renderer,
        application: &'a App,
        parent_position: App::Position,
        screen_clip: App::Clip,
    ) -> ElementRenderer<'a, App> {
        todo!()
    }

    fn hovered_element(&self, mouse_position: App::Position, mouse_mode: &App::MouseInputMode) -> HoverInformation<App> {
        todo!()
    }
}

pub trait DeriveInputGetState<App>
where
    App: Application,
{
    type Type: StateType<App>;

    fn get_state(&self) -> &Self::Type;
    fn get_state_mut(&mut self) -> &mut Self::Type;
}

// Derive: LinkBackDefault
pub trait ElLinkBack<App>
where
    App: Application,
{
    fn link_back(&mut self, weak_self: WeakElementCell<App>, weak_parent: Option<WeakElementCell<App>>);
}

pub trait DeriveInputFocusable<App>
where
    App: Application,
{
    fn is_self_focusable(&self) -> bool;
}

// Derive: FucusableDefault
pub trait ElFocusable<App>
where
    App: Application,
{
    fn is_focusable(&self) -> bool;
}

// Derive: FocusNextDefault
pub trait ElFocusNext<App>
where
    App: Application,
{
    fn focus_next(&self, self_cell: ElementCell<App>, caller_cell: Option<ElementCell<App>>, focus: Focus) -> Option<ElementCell<App>>;
}

// Derive: RestoreFocusDefault
pub trait ElRestoreFocus<App>
where
    App: Application,
{
    fn restore_focus(&self, self_cell: ElementCell<App>) -> Option<ElementCell<App>>;
}

// RestoreFocusDefault
impl<App> ElRestoreFocus<App> for ()
where
    App: Application,
    Self: DeriveInputGetState<App> + DeriveInputFocusable<App>,
{
    fn restore_focus(&self, self_cell: ElementCell<App>) -> Option<ElementCell<App>> {
        <<Self as DeriveInputGetState<App>>::Type as StateType<App>>::restore_focus(self.get_state(), self_cell, self.is_self_focusable())
    }
}

pub trait ElResolve<App>
where
    App: Application,
{
    fn resolve(&mut self, placement_resolver: &mut PlacementResolver<App>, application: &App, theme: &App::Theme);
}

// ResolveAtomic
impl<App> ElResolve<App> for ()
where
    App: Application,
    Self: DeriveInputGetState<App, Type = ElementState<App>> + DeriveInputFocusable<App>,
{
    fn resolve(&mut self, placement_resolver: &mut PlacementResolver<App>, application: &App, theme: &App::Theme) {
        todo!();
    }
}

// ResolveContainer
impl<App> ElResolve<App> for ()
where
    App: Application,
    Self: DeriveInputGetState<App, Type = ContainerState<App>> + DeriveInputFocusable<App>,
{
    fn resolve(&mut self, placement_resolver: &mut PlacementResolver<App>, application: &App, theme: &App::Theme) {
        todo!();
    }
}

// Derive: UpdateBehaviorNone
pub trait ElUpdate<App>
where
    App: Application,
{
    fn update(&mut self) -> Option<ChangeEvent>;
}

// Derive: HoveringDefault
pub trait ElHovering<App>
where
    App: Application,
{
    fn hovered_element(&self, mouse_position: App::Position, mouse_mode: &App::MouseInputMode) -> HoverInformation<App>;
}

// HoveringDefault:
impl<App> ElHovering<App> for ()
where
    App: Application,
    Self: DeriveInputGetState<App> + DeriveInputFocusable<App>,
{
    fn hovered_element(&self, mouse_position: App::Position, mouse_mode: &App::MouseInputMode) -> HoverInformation<App> {
        <<Self as DeriveInputGetState<App>>::Type as StateType<App>>::hovered_element(self.get_state(), mouse_position, mouse_mode)
    }
}

// Derive: LeftClickBehaviorNone
pub trait ElLeftClick<App>
where
    App: Application,
{
    fn left_click(&mut self, update: &mut bool) -> Vec<ClickAction<App>>;
}

// Derive: RightClickBehaviorNone
pub trait ElRightClick<App>
where
    App: Application,
{
    fn right_click(&mut self, update: &mut bool) -> Vec<ClickAction<App>>;
}

// Derive: DragBehaviorNone
pub trait ElDrag<App>
where
    App: Application,
{
    fn drag(&mut self, mouse_delta: App::Position) -> Option<ChangeEvent>;
}

// Derive: ScrollBehaviorNone
pub trait ElScroll<App>
where
    App: Application,
{
    fn scroll(&mut self, delta: f32) -> Option<ChangeEvent>;
}

// Derive: InputBehaviorNone
pub trait ElInputCharacter<App>
where
    App: Application,
{
    fn input_character(&mut self, character: char) -> (Vec<ClickAction<App>>, bool);
}

// Derive: DropBehaviorNone
pub trait ElDropResource<App>
where
    App: Application,
{
    fn drop_resource(&mut self, drop_resource: App::DropResource) -> Option<App::DropResult>;
}

pub trait DeriveInputRender<App>
where
    App: Application,
{
    fn render_inner(
        &self,
        renderer: &mut ElementRenderer<App>,
        theme: &App::Theme,
        hovered_element: Option<&dyn Element<App>>,
        focused_element: Option<&dyn Element<App>>,
        mouse_mode: &App::MouseInputMode,
        second_theme: bool,
    );
}

// Derive: RenderDefault, RenderBare
pub trait ElRender<App>
where
    App: Application,
{
    fn render(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &App::Renderer,
        application: &App,
        theme: &App::Theme,
        parent_position: App::Position,
        screen_clip: App::Clip,
        hovered_element: Option<&dyn Element<App>>,
        focused_element: Option<&dyn Element<App>>,
        mouse_mode: &App::MouseInputMode,
        second_theme: bool,
    );
}

// RenderDefault:
impl<App> ElRender<App> for ()
where
    App: Application,
    Self: DeriveInputGetState<App> + DeriveInputRender<App>,
{
    fn render(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &App::Renderer,
        application: &App,
        theme: &App::Theme,
        parent_position: App::Position,
        screen_clip: App::Clip,
        hovered_element: Option<&dyn Element<App>>,
        focused_element: Option<&dyn Element<App>>,
        mouse_mode: &App::MouseInputMode,
        second_theme: bool,
    ) {
        let mut renderer = self
            .get_state()
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

        self.render_inner(&mut renderer, theme, hovered_element, focused_element, mouse_mode, second_theme);

        <<Self as DeriveInputGetState<App>>::Type as StateType<App>>::render(
            self.get_state(),
            &mut renderer,
            theme,
            hovered_element,
            focused_element,
            mouse_mode,
            second_theme,
        )
    }
}

// RenderBare:
/*impl<App> ElRender<App> for ()
where
    App: Application,
    Self: ElGetState<App>,
{
    fn render(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &App::Renderer,
        application: &App,
        theme: &App::Theme,
        parent_position: App::Position,
        screen_clip: App::Clip,
        hovered_element: Option<&dyn Element<App>>,
        focused_element: Option<&dyn Element<App>>,
        mouse_mode: &App::MouseInputMode,
        second_theme: bool,
    ) {
        let mut renderer = self
            .get_state()
            .element_renderer(render_target, renderer, application, parent_position, screen_clip);

        <<Self as ElGetState<App>>::Type as StateType<App>>::render(
            self.get_state(),
            &mut renderer,
            theme,
            hovered_element,
            focused_element,
            mouse_mode,
            second_theme,
        )
    }
}*/

pub trait Element2<App>
where
    App: Application,
{
    fn link_back(&mut self, weak_self: WeakElementCell<App>, weak_parent: Option<WeakElementCell<App>>);

    fn is_focusable(&self) -> bool;

    fn focus_next(&self, self_cell: ElementCell<App>, caller_cell: Option<ElementCell<App>>, focus: Focus) -> Option<ElementCell<App>>;

    fn restore_focus(&self, self_cell: ElementCell<App>) -> Option<ElementCell<App>>;

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver<App>, application: &App, theme: &App::Theme);

    fn update(&mut self) -> Option<ChangeEvent>;

    fn hovered_element(&self, mouse_position: App::Position, mouse_mode: &App::MouseInputMode) -> HoverInformation<App>;

    fn left_click(&mut self, update: &mut bool) -> Vec<ClickAction<App>>;

    fn right_click(&mut self, update: &mut bool) -> Vec<ClickAction<App>>;

    fn drag(&mut self, mouse_delta: App::Position) -> Option<ChangeEvent>;

    fn scroll(&mut self, delta: f32) -> Option<ChangeEvent>;

    fn input_character(&mut self, character: char) -> (Vec<ClickAction<App>>, bool);

    fn drop_resource(&mut self, drop_resource: App::DropResource) -> Option<App::DropResult>;

    fn render(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &App::Renderer,
        application: &App,
        theme: &App::Theme,
        parent_position: App::Position,
        screen_clip: App::Clip,
        hovered_element: Option<&dyn Element<App>>,
        focused_element: Option<&dyn Element<App>>,
        mouse_mode: &App::MouseInputMode,
        second_theme: bool,
    );
}

impl<T, App> Element2<App> for T
where
    T: ElLinkBack<App>
        + ElFocusable<App>
        + ElFocusNext<App>
        + ElRestoreFocus<App>
        + ElResolve<App>
        + ElUpdate<App>
        + ElHovering<App>
        + ElLeftClick<App>
        + ElRightClick<App>
        + ElDrag<App>
        + ElScroll<App>
        + ElInputCharacter<App>
        + ElDropResource<App>
        + ElRender<App>,
    App: Application,
{
    fn link_back(&mut self, weak_self: WeakElementCell<App>, weak_parent: Option<WeakElementCell<App>>) {
        ElLinkBack::link_back(self, weak_self, weak_parent)
    }

    fn is_focusable(&self) -> bool {
        ElFocusable::is_focusable(self)
    }

    fn focus_next(&self, self_cell: ElementCell<App>, caller_cell: Option<ElementCell<App>>, focus: Focus) -> Option<ElementCell<App>> {
        ElFocusNext::focus_next(self, self_cell, caller_cell, focus)
    }

    fn restore_focus(&self, self_cell: ElementCell<App>) -> Option<ElementCell<App>> {
        ElRestoreFocus::restore_focus(self, self_cell)
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver<App>, application: &App, theme: &<App as Application>::Theme) {
        ElResolve::resolve(self, placement_resolver, application, theme)
    }

    fn update(&mut self) -> Option<ChangeEvent> {
        ElUpdate::update(self)
    }

    fn hovered_element(
        &self,
        mouse_position: <App as Application>::Position,
        mouse_mode: &<App as Application>::MouseInputMode,
    ) -> HoverInformation<App> {
        ElHovering::hovered_element(self, mouse_position, mouse_mode)
    }

    fn left_click(&mut self, update: &mut bool) -> Vec<ClickAction<App>> {
        ElLeftClick::left_click(self, update)
    }

    fn right_click(&mut self, update: &mut bool) -> Vec<ClickAction<App>> {
        ElRightClick::right_click(self, update)
    }

    fn drag(&mut self, mouse_delta: <App as Application>::Position) -> Option<ChangeEvent> {
        ElDrag::drag(self, mouse_delta)
    }

    fn scroll(&mut self, delta: f32) -> Option<ChangeEvent> {
        ElScroll::scroll(self, delta)
    }

    fn input_character(&mut self, character: char) -> (Vec<ClickAction<App>>, bool) {
        ElInputCharacter::input_character(self, character)
    }

    fn drop_resource(&mut self, drop_resource: <App as Application>::DropResource) -> Option<<App as Application>::DropResult> {
        ElDropResource::drop_resource(self, drop_resource)
    }

    fn render(
        &self,
        render_target: &mut <<App as Application>::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &<App as Application>::Renderer,
        application: &App,
        theme: &<App as Application>::Theme,
        parent_position: <App as Application>::Position,
        screen_clip: <App as Application>::Clip,
        hovered_element: Option<&dyn Element<App>>,
        focused_element: Option<&dyn Element<App>>,
        mouse_mode: &<App as Application>::MouseInputMode,
        second_theme: bool,
    ) {
        ElRender::render(
            self,
            render_target,
            renderer,
            application,
            theme,
            parent_position,
            screen_clip,
            hovered_element,
            focused_element,
            mouse_mode,
            second_theme,
        )
    }
}

#[derive(LinkBackDefault, FucusableDefault, FucusNextDefault)]
struct TestElement<App>
where
    App: Application,
{
    element_state: ElementState<App>,
}

impl<App> DeriveInputFocusable<App> for TestElement<App>
where
    App: Application,
{
    fn is_self_focusable(&self) -> bool {
        true
    }
}
