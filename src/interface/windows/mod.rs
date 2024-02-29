mod account;
mod builder;
mod cache;
mod character;
#[cfg(feature = "debug")]
mod debug;
mod friends;
mod generic;
mod mutable;
mod prototype;
mod settings;

pub use self::account::*;
pub use self::builder::WindowBuilder;
pub use self::cache::*;
pub use self::character::*;
#[cfg(feature = "debug")]
pub use self::debug::*;
pub use self::friends::*;
pub use self::generic::*;
pub use self::mutable::*;
pub use self::prototype::PrototypeWindow;
pub use self::settings::*;
use crate::graphics::{InterfaceRenderer, Renderer};
use crate::input::MouseInputMode;
use crate::interface::*;
use crate::loaders::FontLoader;

pub struct Window {
    window_class: Option<String>,
    position: ScreenPosition,
    size_constraint: SizeConstraint,
    size: ScreenSize,
    elements: Vec<ElementCell>,
    popup_element: Option<(ElementCell, Tracker<ScreenPosition>, Tracker<ScreenSize>)>,
    closable: bool,
    background_color: Option<ColorSelector>,
    theme_kind: ThemeKind,
}

impl Window {
    pub fn get_window_class(&self) -> Option<&str> {
        self.window_class.as_deref()
    }

    fn get_background_color(&self, theme: &InterfaceTheme) -> Color {
        self.background_color
            .as_ref()
            .map(|closure| closure(theme))
            .unwrap_or(theme.window.background_color.get())
    }

    pub fn has_transparency(&self, theme: &InterfaceTheme) -> bool {
        const TRANSPARENCY_THRESHOLD: f32 = 0.999;
        self.get_background_color(theme).alpha < TRANSPARENCY_THRESHOLD
    }

    pub fn is_closable(&self) -> bool {
        self.closable
    }

    pub fn get_theme_kind(&self) -> ThemeKind {
        self.theme_kind
    }

    pub fn resolve(
        &mut self,
        font_loader: Rc<RefCell<FontLoader>>,
        interface_settings: &InterfaceSettings,
        theme: &InterfaceTheme,
        available_space: ScreenSize,
    ) -> (Option<&str>, ScreenPosition, ScreenSize) {
        let height = match self.size_constraint.height.is_flexible() {
            true => None,
            false => Some(self.size.height),
        };

        let mut placement_resolver = PlacementResolver::new(
            font_loader.clone(),
            PartialScreenSize::new(self.size.width, height),
            theme.window.border_size.get(),
            theme.window.gaps.get(),
            interface_settings.scaling.get(),
        );

        self.elements
            .iter()
            .for_each(|element| element.borrow_mut().resolve(&mut placement_resolver, interface_settings, theme));

        if self.size_constraint.height.is_flexible() {
            let final_height = theme.window.border_size.get().height + placement_resolver.final_height();
            let final_height = self.size_constraint.validated_height(
                final_height,
                available_space.height.into(),
                available_space.height.into(),
                interface_settings.scaling.get(),
            );
            self.size.height = final_height;
            self.validate_size(interface_settings, available_space);
        }

        self.validate_position(available_space);

        if let Some((popup, _, size_tracker)) = &self.popup_element {
            let size = size_tracker().unwrap(); // FIX: Don't unwrap obviously

            let mut placement_resolver = PlacementResolver::new(
                font_loader,
                PartialScreenSize::new(size.width, Some(200.0)),
                ScreenSize::default(), //theme.window.border_size.get(), // TODO: Popup
                ScreenSize::default(), //theme.window.gaps.get(), // TODO: Popup
                interface_settings.scaling.get(),
            );

            popup.borrow_mut().resolve(&mut placement_resolver, interface_settings, theme);
        };

        (self.window_class.as_deref(), self.position, self.size)
    }

    pub fn update(&mut self) -> Option<ChangeEvent> {
        self.elements
            .iter_mut()
            .map(|element| element.borrow_mut().update())
            .fold(None, |current, other| {
                current.zip_with(other, ChangeEvent::union).or(current).or(other)
            })
    }

    pub fn first_focused_element(&self) -> Option<ElementCell> {
        let element_cell = self.elements[0].clone();
        self.elements[0].borrow().focus_next(element_cell, None, Focus::downwards())
    }

    pub fn restore_focus(&self) -> Option<ElementCell> {
        self.elements[0].borrow().restore_focus(self.elements[0].clone())
    }

    pub fn hovered_element(&self, mouse_position: ScreenPosition, mouse_mode: &MouseInputMode) -> HoverInformation {
        let absolute_position = ScreenPosition::from_size(mouse_position - self.position);

        if let Some((popup, position_tracker, _)) = &self.popup_element {
            let position = position_tracker().unwrap(); // FIX: Don't unwrap obviously
            let position = ScreenPosition::from_size(mouse_position - position);

            match popup.borrow().hovered_element(position, mouse_mode) {
                HoverInformation::Hovered => return HoverInformation::Element(popup.clone()),
                HoverInformation::Missed => {}
                hover_information => return hover_information,
            }
        }

        if absolute_position.left >= 0.0
            && absolute_position.top >= 0.0
            && absolute_position.left <= self.size.width
            && absolute_position.top <= self.size.height
        {
            for element in &self.elements {
                match element.borrow().hovered_element(absolute_position, mouse_mode) {
                    HoverInformation::Hovered => return HoverInformation::Element(element.clone()),
                    HoverInformation::Missed => {}
                    hover_information => return hover_information,
                }
            }

            return HoverInformation::Hovered;
        }

        HoverInformation::Missed
    }

    pub fn get_area(&self) -> (ScreenPosition, ScreenSize) {
        (self.position, self.size)
    }

    pub fn hovers_area(&self, position: ScreenPosition, size: ScreenSize) -> bool {
        let self_combined = self.position + self.size;
        let area_combined = position + size;

        self_combined.left > position.left
            && self.position.left < area_combined.left
            && self_combined.top > position.top
            && self.position.top < area_combined.top
    }

    pub fn offset(&mut self, available_space: ScreenSize, offset: ScreenPosition) -> Option<(&str, ScreenPosition)> {
        self.position += offset;
        self.validate_position(available_space);
        self.window_class
            .as_ref()
            .map(|window_class| (window_class.as_str(), self.position))
    }

    fn validate_position(&mut self, available_space: ScreenSize) {
        self.position = self.size_constraint.validated_position(self.position, self.size, available_space);
    }

    pub fn resize(
        &mut self,
        interface_settings: &InterfaceSettings,
        _theme: &InterfaceTheme,
        available_space: ScreenSize,
        growth: ScreenSize,
    ) -> (Option<&str>, ScreenSize) {
        self.size += growth;
        self.validate_size(interface_settings, available_space);
        (self.window_class.as_deref(), self.size)
    }

    fn validate_size(&mut self, interface_settings: &InterfaceSettings, available_space: ScreenSize) {
        self.size = self
            .size_constraint
            .validated_size(self.size, available_space, interface_settings.scaling.get());
    }

    pub fn open_popup(&mut self, element: ElementCell, position_tracker: Tracker<ScreenPosition>, size_tracker: Tracker<ScreenSize>) {
        // NOTE: Very important to link back
        let weak_element = Rc::downgrade(&element);
        element.borrow_mut().link_back(weak_element, None);

        self.popup_element = Some((element, position_tracker, size_tracker));
    }

    pub fn close_popup(&mut self) {
        self.popup_element = None;
    }

    pub fn render(
        &self,
        render_target: &mut <InterfaceRenderer as Renderer>::Target,
        renderer: &InterfaceRenderer,
        state_provider: &StateProvider,
        interface_settings: &InterfaceSettings,
        theme: &InterfaceTheme,
        hovered_element: Option<&dyn Element>,
        focused_element: Option<&dyn Element>,
        mouse_mode: &MouseInputMode,
    ) {
        let screen_clip = ScreenClip {
            left: self.position.left,
            top: self.position.top,
            right: self.position.left + self.size.width,
            bottom: self.position.top + self.size.height,
        };

        renderer.render_rectangle(
            render_target,
            self.position,
            self.size,
            screen_clip,
            theme.window.corner_radius.get(),
            self.get_background_color(theme),
        );

        self.elements.iter().for_each(|element| {
            element.borrow().render(
                render_target,
                renderer,
                state_provider,
                interface_settings,
                theme,
                self.position,
                screen_clip,
                hovered_element,
                focused_element,
                mouse_mode,
                false,
            )
        });

        if let Some((popup, position_tracker, _)) = &self.popup_element {
            let position = position_tracker().unwrap(); // FIX: Don't unwrap obviously

            popup.borrow().render(
                render_target,
                renderer,
                state_provider,
                interface_settings,
                theme,
                position,
                screen_clip,
                hovered_element,
                focused_element,
                mouse_mode,
                false,
            );
        };
    }
}

// Needed so that we can deallocate FramedWindow in another thread.
unsafe impl Send for Window {}
unsafe impl Sync for Window {}
