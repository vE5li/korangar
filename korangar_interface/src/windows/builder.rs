use std::marker::PhantomData;
use std::rc::Rc;

use super::Window;
use crate::application::{Application, PartialSizeTraitExt, PositionTraitExt, SizeTraitExt, WindowCache};
use crate::builder::{Set, Unset};
use crate::elements::{CloseButtonBuilder, Container, DragButtonBuilder, ElementCell, ElementWrap};
use crate::layout::{Dimension, DimensionBound, SizeBound};
use crate::ColorSelector;

/// Type state [`Window`] builder. This builder utilizes the type system to
/// prevent calling the same method multiple times, calling
/// [`build`](Self::build) before the mandatory methods have been called, and to
/// enforce some conditional logic. Namely, the `closable` method can only be
/// called if the window has a title.
#[must_use = "`build` needs to be called"]
pub struct WindowBuilder<App, Title, Closable, Class, Size, Elements, Background, Theme>
where
    App: Application,
{
    title: Option<String>,
    closable: bool,
    class: Option<String>,
    size_bound: Size,
    elements: Elements,
    background_color: Option<ColorSelector<App>>,
    theme_kind: App::ThemeKind,
    marker: PhantomData<(Title, Closable, Class, Background, Theme)>,
}

impl<App> WindowBuilder<App, Unset, Unset, Unset, Unset, Unset, Unset, Unset>
where
    App: Application,
{
    pub fn new() -> Self {
        Self {
            title: None,
            closable: false,
            class: None,
            size_bound: Unset,
            elements: Unset,
            background_color: None,
            theme_kind: App::ThemeKind::default(),
            marker: PhantomData,
        }
    }
}

impl<App, Class, Closable, Size, Elements, Background, Theme> WindowBuilder<App, Unset, Closable, Class, Size, Elements, Background, Theme>
where
    App: Application,
{
    pub fn with_title(self, title: impl Into<String>) -> WindowBuilder<App, Set, Closable, Class, Size, Elements, Background, Theme> {
        WindowBuilder {
            title: Some(title.into()),
            marker: PhantomData,
            ..self
        }
    }
}

impl<App, Class, Size, Elements, Background, Theme> WindowBuilder<App, Set, Unset, Class, Size, Elements, Background, Theme>
where
    App: Application,
{
    /// NOTE: This function is only available if
    /// [`with_title`](Self::with_title) has been called on the builder.
    pub fn closable(self) -> WindowBuilder<App, Set, Set, Class, Size, Elements, Background, Theme> {
        WindowBuilder {
            closable: true,
            marker: PhantomData,
            ..self
        }
    }
}

impl<App, Title, Closable, Size, Elements, Background, Theme> WindowBuilder<App, Title, Closable, Unset, Size, Elements, Background, Theme>
where
    App: Application,
{
    pub fn with_class(self, class: impl Into<String>) -> WindowBuilder<App, Title, Closable, Set, Size, Elements, Background, Theme> {
        WindowBuilder {
            class: Some(class.into()),
            marker: PhantomData,
            ..self
        }
    }
}

impl<App, Title, Closable, Size, Elements, Background, Theme> WindowBuilder<App, Title, Closable, Unset, Size, Elements, Background, Theme>
where
    App: Application,
{
    pub fn with_class_option(self, class: Option<String>) -> WindowBuilder<App, Title, Closable, Set, Size, Elements, Background, Theme> {
        WindowBuilder {
            class,
            marker: PhantomData,
            ..self
        }
    }
}

impl<App, Title, Closable, Class, Elements, Background, Theme>
    WindowBuilder<App, Title, Closable, Class, Unset, Elements, Background, Theme>
where
    App: Application,
{
    pub fn with_size_bound(
        self,
        size_bound: SizeBound,
    ) -> WindowBuilder<App, Title, Closable, Class, SizeBound, Elements, Background, Theme> {
        WindowBuilder { size_bound, ..self }
    }
}

impl<App, Title, Closable, Class, Size, Background, Theme> WindowBuilder<App, Title, Closable, Class, Size, Unset, Background, Theme>
where
    App: Application,
{
    pub fn with_elements(
        self,
        elements: Vec<ElementCell<App>>,
    ) -> WindowBuilder<App, Title, Closable, Class, Size, Vec<ElementCell<App>>, Background, Theme> {
        WindowBuilder { elements, ..self }
    }
}

impl<App, Title, Closable, Class, Size, Elements, Theme> WindowBuilder<App, Title, Closable, Class, Size, Elements, Unset, Theme>
where
    App: Application,
{
    pub fn with_background_color(
        self,
        background_color: ColorSelector<App>,
    ) -> WindowBuilder<App, Title, Closable, Class, Size, Elements, Set, Theme> {
        WindowBuilder {
            background_color: Some(background_color),
            marker: PhantomData,
            ..self
        }
    }
}

impl<App, Title, Closable, Class, Size, Elements, Background> WindowBuilder<App, Title, Closable, Class, Size, Elements, Background, Unset>
where
    App: Application,
{
    pub fn with_theme_kind(
        self,
        theme_kind: App::ThemeKind,
    ) -> WindowBuilder<App, Title, Closable, Class, Size, Elements, Background, Set> {
        WindowBuilder {
            theme_kind,
            marker: PhantomData,
            ..self
        }
    }
}

impl<App, Title, Closable, Class, Background, Theme>
    WindowBuilder<App, Title, Closable, Class, SizeBound, Vec<ElementCell<App>>, Background, Theme>
where
    App: Application,
{
    /// Take the builder and turn it into a [`Window`].
    ///
    /// NOTE: This method is only available if
    /// [`with_size_bound`](Self::with_size_bound) and
    /// [`with_elements`](Self::with_elements) have been called on the builder.
    pub fn build(self, window_cache: &App::Cache, application: &App, available_space: App::Size) -> Window<App> {
        let Self {
            title,
            closable,
            class,
            size_bound,
            mut elements,
            background_color,
            theme_kind,
            ..
        } = self;

        if closable {
            let close_button = CloseButtonBuilder::new().build().wrap();
            elements.insert(0, close_button);
        }

        if let Some(title) = title {
            // FIX: Any bound will never work properly, use a different way of allocating.
            let width_bound = match closable {
                true => DimensionBound {
                    size: Dimension::Relative(70.0),
                    minimum_size: None,
                    maximum_size: None,
                },
                false => DimensionBound {
                    size: Dimension::Remaining,
                    minimum_size: None,
                    maximum_size: None,
                },
            };

            let drag_button = DragButtonBuilder::new()
                .with_title(title)
                .with_width_bound(width_bound)
                .build()
                .wrap();
            elements.insert(0, drag_button);
        }

        let container_size_bound = SizeBound {
            width: Dimension::Relative(100.0),
            minimum_width: size_bound.minimum_width.map(|_| Dimension::Super),
            maximum_width: size_bound.maximum_width.map(|_| Dimension::Super),
            height: match size_bound.height.is_flexible() {
                true => Dimension::Flexible,
                false => Dimension::Remaining,
            },
            minimum_height: size_bound.minimum_height.map(|_| Dimension::Super),
            maximum_height: size_bound.maximum_height.map(|_| Dimension::Super),
        };
        let elements = vec![Container::new(elements).with_size(container_size_bound).wrap()];

        // Very imporant: give every element a link to its parent to allow propagation
        // of events such as scrolling.
        elements.iter().for_each(|element| {
            let weak_element = Rc::downgrade(element);
            element.borrow_mut().link_back(weak_element, None);
        });

        let (cached_position, cached_size) = class
            .as_ref()
            .and_then(|window_class| window_cache.get_window_state(window_class))
            .unzip();

        let size = cached_size
            .map(|size| size_bound.validated_window_size(size, available_space, application.get_scaling()))
            .unwrap_or_else(|| {
                size_bound
                    .resolve_window::<App::PartialSize>(available_space, available_space, application.get_scaling())
                    .finalize_or(0.0)
            });

        let position = cached_position
            .map(|position| size_bound.validated_position(position, size, available_space))
            .unwrap_or(App::Position::from_size(available_space.shrink(size).halved()));

        Window {
            window_class: class,
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
