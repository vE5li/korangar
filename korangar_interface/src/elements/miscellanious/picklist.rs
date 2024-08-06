use std::cell::RefCell;
use std::rc::Rc;

use rust_state::{Context, Selector, View};

use crate::application::{Application, InterfaceRenderer, MouseInputModeTrait, PositionTraitExt, SizeTraitExt};
use crate::elements::{ButtonBuilder, Element, ElementState, ElementWrap, ScrollView};
use crate::event::{ClickAction, HoverInformation};
use crate::layout::{Dimension, DimensionBound, PlacementResolver, SizeBound};
use crate::theme::ButtonTheme;
use crate::ElementEvent;

pub struct PickList<App, Key, Value, State, Event>
where
    App: Application,
    Key: Clone + AsRef<str> + 'static,
    Value: Clone + PartialEq + 'static,
    State: Selector<App, Value>,
    Event: Clone + ElementEvent<App> + 'static,
{
    options: Vec<(Key, Value)>,
    selected: Option<State>,
    event: Option<Event>,
    width_bound: Option<DimensionBound>,
    state: ElementState<App>,
    latest_position: Rc<RefCell<App::Position>>,
    latest_size: Rc<RefCell<App::Size>>,
}

// HACK: Workaround for Rust incorrect trait bounds when deriving Option<T>
// where T: !Default.
impl<App, Key, Value, State, Event> Default for PickList<App, Key, Value, State, Event>
where
    App: Application,
    Key: Clone + AsRef<str> + 'static,
    Value: Clone + PartialEq + 'static,
    State: Selector<App, Value>,
    Event: Clone + ElementEvent<App> + 'static,
{
    fn default() -> Self {
        Self {
            options: Default::default(),
            selected: Default::default(),
            event: Default::default(),
            width_bound: Default::default(),
            state: Default::default(),
            latest_position: Rc::new(RefCell::new(App::Position::zero())),
            latest_size: Rc::new(RefCell::new(App::Size::zero())),
        }
    }
}

impl<App, Key, Value, State, Event> PickList<App, Key, Value, State, Event>
where
    App: Application,
    Key: Clone + AsRef<str> + 'static,
    Value: Clone + PartialEq + 'static,
    State: Selector<App, Value>,
    Event: Clone + ElementEvent<App> + 'static,
{
    pub fn with_options(mut self, options: Vec<(Key, Value)>) -> Self {
        self.options = options;
        self
    }

    pub fn with_selected(mut self, selected: State) -> Self {
        self.selected = Some(selected);
        self
    }

    pub fn with_event(mut self, event: Event) -> Self {
        self.event = Some(event);
        self
    }

    pub fn with_width(mut self, width_bound: DimensionBound) -> Self {
        self.width_bound = Some(width_bound);
        self
    }
}

impl<App, Key, Value, State, Event> Element<App> for PickList<App, Key, Value, State, Event>
where
    App: Application,
    Key: Clone + AsRef<str> + 'static,
    Value: Clone + PartialEq + 'static,
    State: Selector<App, Value>,
    Event: Clone + ElementEvent<App> + 'static,
{
    fn get_state(&self) -> &ElementState<App> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<App> {
        &mut self.state
    }

    fn resolve(&mut self, state: &View<App>, theme_selector: App::ThemeSelector, placement_resolver: &mut PlacementResolver<App>) {
        let height_bound = *state.get_safe(&ButtonTheme::height_bound(theme_selector));
        let size_bound = self
            .width_bound
            .as_ref()
            .unwrap_or(&DimensionBound::RELATIVE_ONE_HUNDRED)
            .add_height(height_bound);

        self.state.resolve(placement_resolver, &size_bound);
        *self.latest_size.borrow_mut() = self.state.cached_size;
    }

    fn hovered_element(&self, mouse_position: App::Position, mouse_mode: &App::MouseInputMode) -> HoverInformation<App> {
        match mouse_mode.is_none() {
            true => self.state.hovered_element(mouse_position),
            false => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _state: &Context<App>, _force_update: &mut bool) -> Vec<ClickAction<App>> {
        let position_tracker = {
            let latest_position = Rc::downgrade(&self.latest_position);
            move || latest_position.upgrade().map(|position| *position.borrow())
        };

        let size_tracker = {
            let latest_size = Rc::downgrade(&self.latest_size);
            move || latest_size.upgrade().map(|size| *size.borrow())
        };

        let options = self
            .options
            .iter()
            .cloned()
            .map(|(text, option)| {
                // FIX: What is the behavior here when slected is none?
                let selected = self.selected.as_ref().unwrap().clone_inner();
                let mut event = self.event.clone();

                ButtonBuilder::new()
                    .with_text(text)
                    .with_event(move |state: &Context<App>| {
                        state.update_value(&selected, option.clone());

                        let mut actions = vec![ClickAction::ClosePopup];

                        if let Some(event) = &mut event {
                            actions.extend(event.trigger(state));
                        };

                        actions
                    })
                    .build()
                    .wrap()
            })
            .collect();

        let size_bound = SizeBound {
            minimum_height: Some(Dimension::Super),
            maximum_height: Some(Dimension::Super),
            ..SizeBound::only_height(Dimension::Flexible)
        };

        let element = ScrollView::new(options, size_bound)
            .with_background_color(|state, theme_selector| *state.get_safe(&ButtonTheme::background_color(theme_selector)))
            .wrap();

        vec![ClickAction::OpenPopup {
            element,
            position_tracker: Box::new(position_tracker),
            size_tracker: Box::new(size_tracker),
        }]
    }

    fn render(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &App::Renderer,
        state: &View<App>,
        theme_selector: App::ThemeSelector,
        parent_position: App::Position,
        screen_clip: App::Clip,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, state, parent_position, screen_clip);

        let hovered_element = state.get_safe(&App::HoveredElementSelector::default());
        let focused_element = state.get_safe(&App::FocusedElementSelector::default());
        let highlighted = self.is_cell_self(&hovered_element) || self.is_cell_self(&focused_element);

        let background_color = match highlighted {
            true => state.get_safe(&ButtonTheme::hovered_background_color(theme_selector)),
            false => state.get_safe(&ButtonTheme::background_color(theme_selector)),
        };

        renderer.render_background(*state.get_safe(&ButtonTheme::corner_radius(theme_selector)), *background_color);

        *self.latest_position.borrow_mut() = renderer.get_position();

        let foreground_color = match highlighted {
            true => state.get_safe(&ButtonTheme::hovered_foreground_color(theme_selector)),
            false => state.get_safe(&ButtonTheme::foreground_color(theme_selector)),
        };

        // FIX: Don't unwrap. Fix logic
        let selector = self.selected.as_ref().unwrap();
        let current_state = state.get(selector);

        if let Some(current_state) = current_state {
            if let Some((text, _)) = self.options.iter().find(|(_, value)| *value == *current_state) {
                renderer.render_text(
                    text.as_ref(),
                    *state.get_safe(&ButtonTheme::text_offset(theme_selector)),
                    *foreground_color,
                    *state.get_safe(&ButtonTheme::font_size(theme_selector)),
                );
            }
        }
    }
}
