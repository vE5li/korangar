use procedural::dimension;

use crate::interface::*;

#[derive(Default)]
pub struct WindowBuilder {
    window_title: Option<String>,
    window_class: Option<String>,
    size_constraint: Option<SizeConstraint>,
    elements: Vec<ElementCell>,
    closable: bool,
    background_color: Option<ColorSelector>,
    theme_kind: ThemeKind,
}

impl WindowBuilder {
    pub fn with_title(mut self, window_title: String) -> Self {
        self.window_title = Some(window_title);
        self
    }

    pub fn with_class(mut self, window_class: String) -> Self {
        self.window_class = Some(window_class);
        self
    }

    /// To simplify PrototypeWindow proc macro. Migth be removed later
    pub fn with_class_option(self, window_class: Option<String>) -> Self {
        Self { window_class, ..self }
    }

    pub fn with_size(mut self, size_constraint: SizeConstraint) -> Self {
        self.size_constraint = Some(size_constraint);
        self
    }

    pub fn with_elements(self, elements: Vec<ElementCell>) -> Self {
        Self { elements, ..self }
    }

    pub fn with_background_color(mut self, background_color: ColorSelector) -> Self {
        self.background_color = Some(background_color);
        self
    }

    pub fn with_theme_kind(mut self, theme_kind: ThemeKind) -> Self {
        self.theme_kind = theme_kind;
        self
    }

    pub fn closable(mut self) -> Self {
        self.closable = true;
        self
    }

    pub fn build(self, window_cache: &WindowCache, interface_settings: &InterfaceSettings, available_space: ScreenSize) -> Window {
        let WindowBuilder {
            window_title,
            window_class,
            size_constraint,
            mut elements,
            closable,
            background_color,
            theme_kind,
        } = self;

        let size_constraint = size_constraint.expect("window must specify a size constraint");

        if closable {
            assert!(window_title.is_some(), "closable window must also have a title");
            let close_button = CloseButton::default().wrap();
            elements.insert(0, close_button);
        }

        let width_constraint = match closable {
            true => dimension!(70%),
            false => dimension!(!),
        };

        if let Some(title) = window_title {
            let drag_button = DragButton::new(title, width_constraint).wrap();
            elements.insert(0, drag_button);
        }

        let container_constraint = SizeConstraint {
            width: Dimension::Relative(100.0),
            minimum_width: size_constraint.minimum_width.map(|_| Dimension::Super),
            maximum_width: size_constraint.maximum_width.map(|_| Dimension::Super),
            height: Dimension::Flexible,
            minimum_height: size_constraint.minimum_height.map(|_| Dimension::Super),
            maximum_height: size_constraint.maximum_height.map(|_| Dimension::Super),
        };

        let elements = vec![Container::new(elements).with_size(container_constraint).wrap()];

        // very imporant: give every element a link to its parent to allow propagation
        // of events such as scrolling
        elements.iter().for_each(|element| {
            let weak_element = Rc::downgrade(element);
            element.borrow_mut().link_back(weak_element, None);
        });

        let (cached_position, cached_size) = window_class
            .as_ref()
            .and_then(|window_class| window_cache.get_window_state(window_class))
            .unzip();

        let size = cached_size
            .map(|size| size_constraint.validated_window_size(size, available_space, interface_settings.scaling.get()))
            .unwrap_or_else(|| {
                size_constraint
                    .resolve_window(available_space, available_space, interface_settings.scaling.get())
                    .finalize_or(0.0)
            });

        let position = cached_position
            .map(|position| size_constraint.validated_position(position, size, available_space))
            .unwrap_or(ScreenPosition::from_size((available_space - size) / 2.0));

        Window {
            window_class,
            position,
            size_constraint,
            size,
            elements,
            popup_element: None,
            closable,
            background_color,
            theme_kind,
        }
    }
}
