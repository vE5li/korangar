use procedural::{constraint, dimension};

use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::{Element, *};

pub struct PickList<K, T, E>
where
    K: Clone + AsRef<str> + 'static,
    E: Clone + ElementEvent + 'static,
    T: Clone + PartialEq + 'static,
{
    options: Vec<(K, T)>,
    selected: Option<TrackedState<T>>,
    event: Option<E>,
    width_constraint: Option<DimensionConstraint>,
    state: ElementState,
    latest_position: Rc<RefCell<Position>>,
    latest_size: Rc<RefCell<Size>>,
}

// HACK: Workaround for Rust incorrect trait bounds when deriving Option<T>
// where T: !Default.
impl<K, T, E> Default for PickList<K, T, E>
where
    K: Clone + AsRef<str> + 'static,
    E: Clone + ElementEvent + 'static,
    T: Clone + PartialEq + 'static,
{
    fn default() -> Self {
        Self {
            options: Default::default(),
            selected: Default::default(),
            event: Default::default(),
            width_constraint: Default::default(),
            state: Default::default(),
            latest_position: Rc::new(RefCell::new(Position::new(0.0, 0.0))),
            latest_size: Rc::new(RefCell::new(Size::new(0.0, 0.0))),
        }
    }
}

impl<K, T, E> PickList<K, T, E>
where
    K: Clone + AsRef<str> + 'static,
    E: Clone + ElementEvent + 'static,
    T: Clone + PartialEq + 'static,
{
    pub fn with_options(mut self, options: Vec<(K, T)>) -> Self {
        self.options = options;
        self
    }

    pub fn with_selected(mut self, selected: TrackedState<T>) -> Self {
        self.selected = Some(selected);
        self
    }

    pub fn with_event(mut self, event: E) -> Self {
        self.event = Some(event);
        self
    }

    pub fn with_width(mut self, width_constraint: DimensionConstraint) -> Self {
        self.width_constraint = Some(width_constraint);
        self
    }
}

impl<K, T, E> Element for PickList<K, T, E>
where
    K: Clone + AsRef<str> + 'static,
    E: Clone + ElementEvent + 'static,
    T: Clone + PartialEq + 'static,
{
    fn get_state(&self) -> &ElementState {
        &self.state
    }

    fn get_state_mut(&mut self) -> &mut ElementState {
        &mut self.state
    }

    fn resolve(&mut self, placement_resolver: &mut PlacementResolver, _interface_settings: &InterfaceSettings, theme: &InterfaceTheme) {
        let size_constraint = self
            .width_constraint
            .as_ref()
            .unwrap_or(&dimension!(100%))
            .add_height(theme.button.height_constraint);

        self.state.resolve(placement_resolver, &size_constraint);

        *self.latest_size.borrow_mut() = self.state.cached_size;
    }

    fn hovered_element(&self, mouse_position: Position, mouse_mode: &MouseInputMode) -> HoverInformation {
        match mouse_mode {
            MouseInputMode::None => self.state.hovered_element(mouse_position),
            _ => HoverInformation::Missed,
        }
    }

    fn left_click(&mut self, _force_update: &mut bool) -> Vec<ClickAction> {
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

                Button::default()
                    .with_text(text)
                    .with_event(Box::new(move || {
                        selected.set(option.clone());
                        let mut actions = vec![ClickAction::ClosePopup];

                        if let Some(event) = &mut event {
                            actions.extend(event.trigger());
                        };

                        actions
                    }))
                    .wrap()
            })
            .collect();

        let element = ScrollView::new(options, constraint!(100%, ? < 500))
            .with_background_color(|theme| *theme.button.background_color)
            .wrap();

        vec![ClickAction::OpenPopup {
            element,
            position_tracker: Box::new(position_tracker),
            size_tracker: Box::new(size_tracker),
        }]
    }

    fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        _state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &InterfaceTheme,
        parent_position: Position,
        clip_size: ClipSize,
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        _mouse_mode: &MouseInputMode,
        _second_theme: bool,
    ) {
        let mut renderer = self
            .state
            .element_renderer(render_target, renderer, interface_settings, parent_position, clip_size);

        let highlighted = self.is_element_self(hovered_element) || self.is_element_self(focused_element);
        let background_color = match highlighted {
            true => *theme.button.hovered_background_color,
            false => *theme.button.background_color,
        };

        renderer.render_background(*theme.button.border_radius, background_color);

        *self.latest_position.borrow_mut() = renderer.get_position();

        let foreground_color = match highlighted {
            true => *theme.button.hovered_foreground_color,
            false => *theme.button.foreground_color,
        };

        // FIX: Don't unwrap. Fix logic
        let current_state = self.selected.as_ref().map(|state| state.get()).unwrap();

        if let Some((text, _)) = self.options.iter().find(|(_, value)| *value == current_state) {
            renderer.render_text(
                text.as_ref(),
                *theme.button.text_offset,
                foreground_color,
                *theme.button.font_size,
            );
        }
    }
}
