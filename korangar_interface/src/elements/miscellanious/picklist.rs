use std::cell::RefCell;
use std::rc::Rc;

use crate::application::{Application, MouseInputModeTrait, PositionTraitExt, SizeTraitExt};
use crate::elements::{ButtonBuilder, Element, ElementState, ElementWrap, ScrollView};
use crate::event::{ClickAction, HoverInformation};
use crate::layout::{Dimension, DimensionBound, PlacementResolver, SizeBound};
use crate::state::{TrackedState, TrackedStateClone};
use crate::theme::{ButtonTheme, InterfaceTheme};
use crate::ElementEvent;

pub struct PickList<App, Key, Value, State, Event>
where
    App: Application,
    Key: Clone + AsRef<str> + 'static,
    Value: Clone + PartialEq + 'static,
    State: TrackedState<Value> + 'static,
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
    State: TrackedState<Value> + 'static,
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
    State: TrackedState<Value> + 'static,
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
    State: TrackedState<Value> + 'static,
    Event: Clone + ElementEvent<App> + 'static,
{
    fn get_state(&self) -> &ElementState<App> {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState<App> {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver<App>, _application: &App, theme: &App::Theme) {
        let size_bound = self
            .width_bound
            .as_ref()
            .unwrap_or(&DimensionBound::RELATIVE_ONE_HUNDRED)
            .add_height(theme.button().height_bound());

        self.state.resolve(placement_resolver, &size_bound);

        *self.latest_size.borrow_mut() = self.state.cached_size;
    }

    fn hovered_element(&self, mouse_position: App::Position, mouse_mode: &App::MouseInputMode) -> HoverInformation<App> {
        match mouse_mode.is_none() {
            true => self.state.hovered_element(mouse_position),
            false => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Vec<ClickAction<App>> {
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
                let mut selected = self.selected.clone().unwrap();
                let mut event = self.event.clone();

                ButtonBuilder::new()
                    .with_text(text)
                    .with_event(Box::new(move || {
                        selected.set(option.clone());
                        let mut actions = vec![ClickAction::ClosePopup];

                        if let Some(event) = &mut event {
                            actions.extend(event.trigger());
                        };

                        actions
                    }))
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
            .with_background_color(|theme| theme.button().background_color())
            .wrap();

        vec![ClickAction::OpenPopup {
            element,
            position_tracker: Box::new(position_tracker),
            size_tracker: Box::new(size_tracker),
        }]
    }

    fn render(
        &self,
        renderer: &App::Renderer,
        application: &App,
        theme: &App::Theme,
        parent_position: App::Position,
        screen_clip: App::Clip,
        hovered_element: Option<&dyn Element<App>>,
        focused_element: Option<&dyn Element<App>>,
        _mouse_mode: &App::MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self.state.element_renderer(renderer, application, parent_position, screen_clip);

        let highlighted = self.is_element_self(hovered_element) || self.is_element_self(focused_element);
        let background_color = match highlighted {
            true => theme.button().hovered_background_color(),
            false => theme.button().background_color(),
        };

        renderer.render_background(theme.button().corner_radius(), background_color);

        *self.latest_position.borrow_mut() = renderer.get_position();

        let foreground_color = match highlighted {
            true => theme.button().hovered_foreground_color(),
            false => theme.button().foreground_color(),
        };

        // FIX: Don't unwrap. Fix logic
        let current_state = self.selected.as_ref().map(|state| state.cloned()).unwrap();

        if let Some((text, _)) = self.options.iter().find(|(_, value)| *value == current_state) {
            renderer.render_text(
                text.as_ref(),
                theme.button().text_offset(),
                foreground_color,
                theme.button().font_size(),
            );
        }
    }
}
