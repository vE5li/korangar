use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use rust_state::{Context, DynSelector, ReadState, RustState, SafeUnwrap, Selector, View};

use crate::application::{
    Application, ClipTrait, CornerRadiusTraitExt, FontSizeTraitExt, InterfaceRenderer, MouseInputModeTrait, PartialSizeTraitExt,
    PositionTrait, PositionTraitExt, SizeTrait, SizeTraitExt,
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
    pub state: &'a View<'b, App>,
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
        application: &View<App>,
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
        state: &'a View<'b, App>,
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

    fn resolve(&mut self, application: &View<App>, theme_selector: App::ThemeSelector, placement_resolver: &mut PlacementResolver<App>);

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
        application: &View<App>,
        theme_selector: App::ThemeSelector,
        parent_position: App::Position,
        screen_clip: App::Clip,
        second_theme: bool,
    );
}

////////////////////////////////

#[derive(RustState)]
#[state_root]
pub struct Element2State<App>
where
    App: Application,
{
    pub cached_size: App::Size,
    pub cached_position: App::Position,
    pub self_element: ElementHandle,
    pub parent_element: Option<ElementHandle>,
    // pub mouse_position: Cell<App::Position>,
    // pub focus_cache: Cell<Option<usize>>,
    pub elements: Vec<ElementHandle>,
    pub __custom: Option<Box<dyn std::any::Any>>,
}

#[derive(Clone)]
pub struct CustomBoxSelector;

impl<App> Element2State<App>
where
    App: Application,
{
    pub fn new(
        self_element: ElementHandle,
        parent_element: Option<ElementHandle>,
        custom: Option<Box<dyn std::any::Any>>,
    ) -> Context<Self> {
        Context::new(Self {
            cached_size: App::Size::new(0.0, 0.0),
            cached_position: App::Position::new(0.0, 0.0),
            self_element,
            parent_element,
            elements: Vec::new(),
            __custom: custom,
        })
    }

    pub fn custom() -> CustomBoxSelector {
        CustomBoxSelector
    }
}

impl<App: Application, To> Selector<Element2State<App>, To> for CustomBoxSelector {
    fn get<'a>(&self, state: &'a Element2State<App>) -> Option<&'a To> {
        state.__custom?.downcast_ref::<To>()
    }

    fn get_mut<'a>(&self, state: &'a mut Element2State<App>) -> Option<&'a mut To> {
        state.__custom?.downcast_mut::<To>()
    }

    fn get_path_id(&self) -> rust_state::PathId {
        Element2State::<App>::__custom().get_path_id()
    }

    fn clone_inner(&self) -> Self
    where
        Self: Sized,
    {
        self.clone()
    }
}

impl !SafeUnwrap for CustomBoxSelector {}

pub enum Resolve<App: Application> {
    Default(fn(&World<App>) -> SizeBound),
    Custom(fn(&World<App>, &mut PlacementResolver<App>)),
}

pub enum Focusable<App: Application> {
    Yes,
    No,
    Dynamic(fn(&World<App>) -> bool),
}

pub enum HoverCheck<App: Application> {
    Default,
    Custom(
        fn(
            &Context<Element2State<App>>,
            &Context<App>,
            App::Position,
            &App::MouseInputMode,
            &ElementAllocator<App>,
        ) -> HoverInformation<App>,
    ),
}

pub enum ModeCheck<App: Application> {
    Default,
    Custom(fn(&Context<Element2State<App>>, &Context<App>, &App::MouseInputMode) -> bool),
}

pub struct World<'a, App: Application> {
    pub this: View<'a, Element2State<App>>,
    pub global: View<'a, App>,
    pub theme_selector: App::ThemeSelector,
}

impl<'a, App: Application> World<'a, App> {
    pub fn evaluator<EvaluatorSelector, Evaluator, To>(&self, selector: &EvaluatorSelector) -> To
    where
        EvaluatorSelector: Selector<Element2State<App>, Evaluator>,
        Evaluator: Fn(&World<App>) -> To,
    {
        let evaluator = self.this.get(selector).unwrap();
        evaluator(self)
    }

    pub fn evaluator_option<EvaluatorSelector, Evaluator, To>(&self, selector: &EvaluatorSelector) -> Option<To>
    where
        EvaluatorSelector: Selector<Element2State<App>, Option<Evaluator>>,
        Evaluator: Fn(&World<App>) -> To,
    {
        self.this.get(selector).unwrap().map(|evaluator| evaluator(self))
    }

    pub fn evaluator_option_fallback<EvaluatorSelector, FallbackSelector, Evaluator, To>(
        &self,
        selector: &EvaluatorSelector,
        fallback: &FallbackSelector,
    ) -> To
    where
        EvaluatorSelector: Selector<Element2State<App>, Option<Evaluator>>,
        FallbackSelector: Selector<App, To> + SafeUnwrap,
        Evaluator: Fn(&World<App>) -> To,
        To: Clone,
    {
        self.this
            .get(selector)
            .unwrap()
            .map(|evaluator| evaluator(self))
            .unwrap_or_else(|| self.global.get_safe(fallback).clone())
    }
}

/* pub trait OptionWorldExt<T> {
    fn unwrap_or_selector<App: Application>(self, world: &World<App>, selector: &(impl Selector<App, T> + SafeUnwrap)) -> T;
}

impl<T> OptionWorldExt<T> for Option<T>
where
    T: Clone,
{
    fn unwrap_or_selector<App: Application>(self, world: &World<App>, selector: &(impl Selector<App, T> + SafeUnwrap)) -> T {
        self.unwrap_or_else(|| world.global.get_safe(selector).clone())
    }
} */

pub type ClickHandler<App: Application> = fn(&World<App>) -> ();
pub type InputHandler<App: Application> = fn(&World<App>, character: char) -> ();
pub type ResourceHandler<App: Application> = fn(&World<App>, resource: App::DropResource) -> Option<App::DropResult>;
pub type ScrollHandler<App: Application> = fn(&World<App>, delta: f32) -> ();
pub type RenderFunction<App: Application> = fn(&World<App>, &mut ElementRenderer<App>);

type Function<App: Application> = fn(&World<App>) -> Vec<ClickAction<App>>;
type Procedure<App: Application> = fn(&World<App>);

pub struct VTable<App: Application> {
    pub on_initialize: Option<fn(&World<App>, ElementManager<App>)>,
    pub on_is_focusable: Focusable<App>,
    pub on_resolve: Resolve<App>,

    pub hover_check: HoverCheck<App>,
    pub mode_check: ModeCheck<App>,

    pub on_left_click: Option<Function<App>>,
    pub on_right_click: Option<Function<App>>,
    pub on_drag: Option<Function<App>>,
    pub on_input_character: Option<InputHandler<App>>,
    pub on_drop_resource: Option<ResourceHandler<App>>,
    pub on_scroll: Option<ScrollHandler<App>>,
    pub background: Option<fn(&World<App>) -> (App::Color, App::CornerRadius)>,
    pub render: RenderFunction<App>,
}

#[derive(Default)]
pub struct ElementReadStates {
    initialize_read_this: ReadState,
    initialize_read_state: ReadState,
    resolve_read_this: ReadState,
    resolve_read_state: ReadState,
    render_read_this: ReadState,
    render_read_state: ReadState,
}

pub struct Element2<App>
where
    App: Application,
{
    vtable: &'static VTable<App>,
    state: Context<Element2State<App>>,
    read_state: RefCell<ElementReadStates>,
}

impl<App: Application> Element2<App> {
    pub fn new(
        vtable: &'static VTable<App>,
        custom: Option<Box<dyn std::any::Any>>,
        state: &Context<App>,
        allocator: &mut ElementAllocator<App>,
        parent_handle: Option<ElementHandle>,
        theme_selector: App::ThemeSelector,
    ) -> ElementHandle {
        let handle = {
            let handle = allocator.allocate(move |handle| Self {
                vtable,
                state: Element2State::new(handle, parent_handle, custom),
                read_state: RefCell::new(ElementReadStates::default()),
            });
            let element = allocator.get(&handle).unwrap();
            let element_manager = ElementManager::new(allocator, &element.state);

            if let Some(initialize) = vtable.on_initialize {
                let mut read_states = element.read_state.borrow_mut();
                let this_tracker = read_states.initialize_read_this.track(&element.state);
                let state_tracker = read_states.initialize_read_this.track(state);

                let world = World {
                    this: this_tracker,
                    global: state_tracker,
                    theme_selector,
                };

                initialize(&world, element_manager);
            }

            handle
        };

        let element = allocator.get_mut(&handle).unwrap();
        element.state.apply();

        handle
    }

    pub fn focus_next(
        &self,
        self_handle: ElementHandle,
        caller_handle: Option<ElementHandle>,
        focus: Focus,
        allocator: &ElementAllocator<App>,
    ) -> Option<ElementHandle> {
        if focus.downwards {
            return Some(self_handle);
        }

        self.state
            .get_safe(&Element2State::<App>::parent_element())
            .and_then(|parent_handle| {
                let parent_element = allocator.get(&parent_handle).unwrap();
                parent_element.focus_next(parent_handle, Some(self_handle), focus, allocator)
            })
    }

    fn restore_focus(&self, self_handle: ElementHandle) -> Option<ElementHandle> {
        // if let Some(index) = self.focus_cache.get()
        //     && !self.elements.is_empty()
        // {
        //     let focused_element =
        // self.elements[0..index.add(1).min(self.elements.len())]
        //         .iter()
        //         .rev()
        //         .find_map(|element| element.borrow().restore_focus(element.clone()));
        //
        //     if focused_element.is_some() {
        //         return focused_element;
        //     }
        // }
        //
        // // TODO: only if focusable
        // Some(self_cell)
        None
    }

    pub fn hovered_element(
        &self,
        state: &Context<App>,
        mouse_position: App::Position,
        mouse_mode: &App::MouseInputMode,
        allocator: &ElementAllocator<App>,
    ) -> Option<ElementHandle> {
        match self.vtable.hover_check {
            HoverCheck::Default => {
                let cached_position = self.state.get_safe(&Element2State::<App>::cached_position());
                let cached_size = self.state.get_safe(&Element2State::<App>::cached_size());
                let elements = self.state.get_safe(&Element2State::<App>::elements());

                let absolute_position = mouse_position.relative_to(*cached_position);

                if absolute_position.left() >= 0.0
                    && absolute_position.top() >= 0.0
                    && absolute_position.left() <= cached_size.width()
                    && absolute_position.top() <= cached_size.height()
                {
                    for handle in elements {
                        let element = allocator.get(handle).unwrap();

                        if let Some(handle) = element.hovered_element(state, absolute_position, mouse_mode, allocator) {
                            return Some(handle);
                        }
                    }

                    // for containers
                    if
                    /* hoverable */
                    true {
                        let self_handle = self.state.get_safe(&Element2State::<App>::self_element()).clone();

                        match self.vtable.mode_check {
                            ModeCheck::Default => {
                                if mouse_mode.is_none() {
                                    return Some(self_handle);
                                }
                            }
                            ModeCheck::Custom(checker) => {
                                if checker(&self.state, state, mouse_mode) {
                                    return Some(self_handle);
                                }
                            }
                        }
                    }
                }

                None
            }
            HoverCheck::Custom(_) => todo!(),
        }
    }

    pub fn check_needs_render(&self, state: &Context<App>) -> bool {
        false
    }

    pub fn resolve(&mut self, state: &Context<App>, theme_selector: App::ThemeSelector, placement_resolver: &mut PlacementResolver<App>) {
        let mut read_states = self.read_state.borrow_mut();
        let this_tracker = read_states.resolve_read_this.track(&self.state);
        let state_tracker = read_states.resolve_read_this.track(state);

        let world = World {
            this: this_tracker,
            global: state_tracker,
            theme_selector,
        };

        let size_bound = (self.vtable.size_bound)(&world);

        let (size, position) = placement_resolver.allocate(&size_bound);
        // self.cached_size = size.finalize();
        // self.cached_position = position;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ElementVersion(usize);

impl ElementVersion {
    pub fn increment(&self) -> Self {
        Self(self.0.wrapping_add(1))
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ElementHandle {
    version: ElementVersion,
    index: usize,
}

enum ElementSlot<App: Application> {
    Used { version: ElementVersion, element: Element2<App> },
    Free { version: ElementVersion },
}

pub struct ElementAllocator<App: Application> {
    slots: Vec<ElementSlot<App>>,
}

impl<App: Application> ElementAllocator<App> {
    pub fn allocate(&mut self, f: impl Fn(ElementHandle) -> Element2<App>) -> ElementHandle {
        for (index, slot) in self.slots.iter_mut().enumerate() {
            let ElementSlot::Free { version } = slot else {
                continue;
            };
            let version = *version;

            let handle = ElementHandle { index, version };
            let element = f(handle.clone());
            *slot = ElementSlot::Used { version, element };

            return handle;
        }

        let index = self.slots.len();
        let version = ElementVersion(0);
        let handle = ElementHandle { index, version };

        let element = f(handle.clone());
        self.slots.push(ElementSlot::Used { version, element });

        handle
    }

    pub fn deallocate(&mut self, handle: ElementHandle) {
        let Some(slot) = self.slots.get_mut(handle.index) else {
            return;
        };

        let (version, child_handles) = {
            let ElementSlot::Used { version, element } = slot else {
                return;
            };

            if *version != handle.version {
                return;
            }

            let element_handles = element.state.get_safe(&Element2State::<App>::elements()).iter().collect::<Vec<_>>();
            (version, element_handles)
        };

        *slot = ElementSlot::Free {
            version: version.increment(),
        }
    }

    pub fn get(&self, handle: &ElementHandle) -> Option<&Element2<App>> {
        let slot = self.slots.get(handle.index)?;

        let ElementSlot::Used { version, element } = slot else {
            return None;
        };

        if *version != handle.version {
            return None;
        }

        Some(element)
    }

    pub fn get_mut(&mut self, handle: &ElementHandle) -> Option<&mut Element2<App>> {
        let slot = self.slots.get_mut(handle.index)?;

        let ElementSlot::Used { version, element } = slot else {
            return None;
        };

        if *version != handle.version {
            return None;
        }

        Some(element)
    }
}

pub struct ElementManager<'a, App: Application> {
    allocator: &'a mut ElementAllocator<App>,
    state: &'a Context<Element2State<App>>,
}

impl<'a, App: Application> ElementManager<'a, App> {
    pub fn new(allocator: &'a mut ElementAllocator<App>, state: &'a Context<Element2State<App>>) -> Self {
        Self { allocator, state }
    }

    pub fn add(&mut self, handle: ElementHandle) {
        self.state
            .update_value_with(&Element2State::<App>::elements(), move |elements| elements.push(handle));
    }

    pub fn remove(&mut self, handle: ElementHandle) {
        self.allocator.deallocate(handle.clone());
        self.state.update_value_with(&Element2State::<App>::elements(), move |elements| {
            elements.retain(|element| *element != handle)
        });
    }

    pub fn iter(&self) -> std::slice::Iter<'a, ElementHandle> {
        self.state.get_safe(&Element2State::<App>::elements()).iter()
    }
}

pub trait PrototypeSelectorElement<App>
where
    App: Application,
{
    fn to_selected_element(
        state: &Context<App>,
        selector: DynSelector<App, Self>,
        allocator: &mut ElementAllocator<App>,
        parent_handle: Option<ElementHandle>,
        theme_selector: App::ThemeSelector,
    ) -> ElementHandle
    where
        Self: Sized;
}

mod vec_container {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::marker::PhantomData;

    use rust_state::{Context, DynSelector, RustState, Selector, SelectorExt, VecItem, VecLookup, View};

    use super::{
        Element2, Element2State, ElementAllocator, ElementHandle, ElementManager, ElementRenderer, Focusable, HoverCheck, ModeCheck,
        PrototypeSelectorElement, Resolve, World,
    };
    use crate::application::{Application, InterfaceRenderer, PartialSizeTraitExt};
    use crate::elements::base::VTable;
    use crate::layout::{Dimension, PlacementResolver, SizeBound};
    use crate::theme::ExpandableTheme;

    #[derive(RustState)]
    struct VecState<App, Item>
    where
        App: Application,
        Item: VecItem + 'static,
    {
        selector: DynSelector<App, Vec<Item>>,
        associated_ids: RefCell<HashMap<ElementHandle, Item::Id>>,
        expanded: bool,
        _marker: PhantomData<(App, Item)>,
    }

    fn background_color_thing<App: Application>(world: &World<App>) -> (App::Color, App::CornerRadius) {
        let color = world
            .global
            .get_safe(&ExpandableTheme::<App>::background_color(world.theme_selector))
            .clone();
        let corner_radius = world
            .global
            .get_safe(&ExpandableTheme::<App>::corner_radius(world.theme_selector))
            .clone();

        (color, corner_radius)
    }

    fn specific_resolve<App, Item>(world: &World<App>)
    where
        App: Application,
        Item: PrototypeSelectorElement<App> + VecItem,
    {
        let state: &VecState<App, Item> = world.this.get(&Element2State::<App>::custom()).unwrap();
    }

    fn size_bound<App, Item>(world: &World<App>, placement_resolver: &mut PlacementResolver<App>) -> SizeBound
    where
        App: Application,
        Item: VecItem + 'static,
    {
        let elements = *world.this.get(&Element2State::<App>::elements()).unwrap();
        let expanded = *world
            .this
            .get(&VecState::<App, Item>::expanded(Element2State::<App>::custom()))
            .unwrap();

        let closed_size_bound = SizeBound::only_height(Dimension::Absolute(18.0));

        let closed_size = closed_size_bound
            .resolve_element::<App::PartialSize>(
                placement_resolver.get_available(),
                placement_resolver.get_remaining(),
                &placement_resolver.get_parent_limits(),
                *world.global.get_safe(&App::ScaleSelector::default()),
            )
            .finalize::<App::Size>();

        match expanded && !elements.is_empty() {
            true => SizeBound::only_height(Dimension::Flexible),
            false => closed_size_bound,
        }
    }

    fn initialize<App, Item>(world: &World<App>, mut elements: ElementManager<App>)
    where
        App: Application,
        Item: PrototypeSelectorElement<App> + VecItem,
    {
        let custom_selector = Element2State::<App>::custom();

        // TODO: add some debug message+icon if the state is no longer available.
        let Some(associated) = world.this.get(&VecState::<App, Item>::associated_ids(custom_selector.clone())) else {
            return;
        };
        let Some(selector) = world.this.get(&VecState::<App, Item>::selector(custom_selector.clone())) else {
            return;
        };
        let Some(items) = world.global.get(selector) else {
            return;
        };

        let self_handle = world.this.get_safe(&Element2State::<App>::self_element());

        for handle in elements.iter() {
            let associated_id = associated.borrow().get(handle).unwrap();

            if !items.iter().any(|item| item.get_id() == *associated_id) {
                elements.remove(handle.clone());
            }
        }

        for item in items {
            // if associated.borrow().get(handle).is_none() {
            let item_id = item.get_id();
            let item_selector = VecLookup::new(selector.clone_inner(), item_id.clone());
            let element_handle = Item::to_selected_element(
                world.global.get_context(),
                item_selector.to_dyn(),
                elements.allocator,
                Some(self_handle.clone()),
                world.theme_selector,
            );

            elements.add(element_handle.clone());
            associated.borrow_mut().insert(element_handle, item_id);
        }
        // }
    }

    fn render<App>(world: &World<App>, renderer: Element2Renderer<App>)
    where
        App: Application,
    {
    }

    impl<App, Item> PrototypeSelectorElement<App> for Vec<Item>
    where
        App: Application,
        Item: PrototypeSelectorElement<App> + VecItem,
    {
        fn to_selected_element(
            state: &Context<App>,
            selector: DynSelector<App, Self>,
            allocator: &mut ElementAllocator<App>,
            parent_handle: Option<ElementHandle>,
            theme_selector: App::ThemeSelector,
        ) -> ElementHandle {
            let vtable = const {
                &VTable {
                    on_initialize: Some(initialize::<App, Item>),
                    on_is_focusable: Focusable::No,
                    on_resolve: Resolve::Custom(size_bound::<App, Item>),
                    hover_check: HoverCheck::Default,
                    mode_check: ModeCheck::Default,
                    on_left_click: None,
                    on_right_click: None,
                    on_drag: None,
                    on_input_character: None,
                    on_drop_resource: None,
                    on_scroll: None,
                    background: Some(background_color_thing::<App>),
                    render: render::<App>,
                }
            };

            let custom = Box::new(VecState {
                expanded: false,
                selector,
                associated_ids: RefCell::new(HashMap::new()),
                _marker: PhantomData,
            });

            Element2::new(vtable, Some(custom), state, allocator, parent_handle, theme_selector)
        }
    }
}
