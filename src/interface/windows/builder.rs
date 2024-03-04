use procedural::dimension_bound;

use crate::interface::*;

#[derive(Default)]
pub struct WindowBuilder {
    window_title: Option<String>,
    window_class: Option<String>,
    size_bound: Option<SizeBound>,
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

    pub fn with_size(mut self, size_bound: SizeBound) -> Self {
        self.size_bound = Some(size_bound);
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
            size_bound,
            mut elements,
            closable,
            background_color,
            theme_kind,
        } = self;

        let size_bound = size_bound.expect("window must specify a size bound");

        if closable {
            assert!(window_title.is_some(), "closable window must also have a title");
            let close_button = CloseButton::default().wrap();
            elements.insert(0, close_button);
        }

        let width_bound = match closable {
            true => dimension_bound!(70%),
            false => dimension_bound!(!),
        };

        if let Some(title) = window_title {
            let drag_button = DragButton::new(title, width_bound).wrap();
            elements.insert(0, drag_button);
        }

        let container_size_bound = SizeBound {
            width: Dimension::Relative(100.0),
            minimum_width: size_bound.minimum_width.map(|_| Dimension::Super),
            maximum_width: size_bound.maximum_width.map(|_| Dimension::Super),
            height: Dimension::Flexible,
            minimum_height: size_bound.minimum_height.map(|_| Dimension::Super),
            maximum_height: size_bound.maximum_height.map(|_| Dimension::Super),
        };

        let elements = vec![Container::new(elements).with_size(container_size_bound).wrap()];

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
            .map(|size| size_bound.validated_window_size(size, available_space, interface_settings.scaling.get()))
            .unwrap_or_else(|| {
                size_bound
                    .resolve_window(available_space, available_space, interface_settings.scaling.get())
                    .finalize_or(0.0)
            });

        let position = cached_position
            .map(|position| size_bound.validated_position(position, size, available_space))
            .unwrap_or(ScreenPosition::from_size((available_space - size) / 2.0));

        Window {
            window_class,
            position,
            size_bound,
            size,
            elements,
            popup_element: None,
            closable,
            background_color,
            theme_kind,
        }
    }
}
