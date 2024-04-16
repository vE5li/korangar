use std::rc::{Rc, Weak};

use crate::elements::{Element, ElementCell, WeakElementCell};
use crate::theme::InterfaceTheme;

pub trait Application: Sized + 'static {
    type ThemeKind: Default;
    type Theme: InterfaceTheme<Settings = Self>;
    type Color: ColorTrait;
    type Renderer: InterfaceRenderer<Self>;
    type Size: SizeTrait;
    type PartialSize: PartialSizeTrait;
    type Position: PositionTrait;
    type Clip: ClipTrait;
    type CornerRadius: CornerRadiusTrait;
    type FontSize: FontSizeTrait;
    type Scaling: ScalingTrait;
    type FontLoader: FontLoaderTrait<Self>;
    type MouseInputMode: MouseInputModeTrait<Self>;
    type Cache: WindowCache<Self>;
    type DropResource;
    type DropResult;
    type CustomEvent;

    fn get_scaling(&self) -> Self::Scaling;

    fn get_theme(&self, kind: &Self::ThemeKind) -> &Self::Theme;
}

pub trait MouseInputModeTrait<App>
where
    App: Application,
{
    fn is_none(&self) -> bool;

    fn is_self_dragged(&self, element: &dyn Element<App>) -> bool;
}

pub trait FontSizeTrait: Copy {
    fn new(value: f32) -> Self;

    fn get_value(&self) -> f32;
}

pub trait FontSizeTraitExt {
    fn scaled(&self, scaling: impl ScalingTrait) -> Self;
}

impl<T> FontSizeTraitExt for T
where
    T: FontSizeTrait,
{
    fn scaled(&self, scaling: impl ScalingTrait) -> Self {
        Self::new(self.get_value() * scaling.get_factor())
    }
}

pub trait FontLoaderTrait<App>: Clone
where
    App: Application,
{
    fn get_text_dimensions(&self, text: &str, font_size: App::FontSize, available_width: f32) -> App::Size;
}

pub trait ScalingTrait: Copy {
    fn get_factor(&self) -> f32;
}

pub trait InterfaceRenderer<App>
where
    App: Application,
{
    type Target;

    fn get_text_dimensions(&self, text: &str, font_size: App::FontSize, available_width: f32) -> App::Size;

    fn render_rectangle(
        &self,
        render_target: &mut Self::Target,
        position: App::Position,
        size: App::Size,
        clip: App::Clip,
        corner_radius: App::CornerRadius,
        color: App::Color,
    );

    fn render_text(
        &self,
        render_target: &mut Self::Target,
        text: &str,
        position: App::Position,
        clip: App::Clip,
        color: App::Color,
        font_size: App::FontSize,
    ) -> f32;

    fn render_checkbox(
        &self,
        render_target: &mut Self::Target,
        position: App::Position,
        size: App::Size,
        clip: App::Clip,
        color: App::Color,
        checked: bool,
    );

    fn render_expand_arrow(
        &self,
        render_target: &mut Self::Target,
        position: App::Position,
        size: App::Size,
        clip: App::Clip,
        color: App::Color,
        expanded: bool,
    );
}

pub trait ColorTrait: Clone {
    fn is_transparent(&self) -> bool;
}

pub trait CornerRadiusTrait {
    fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self;

    fn top_left(&self) -> f32;

    fn top_right(&self) -> f32;

    fn bottom_right(&self) -> f32;

    fn bottom_left(&self) -> f32;
}

pub trait CornerRadiusTraitExt {
    fn zero() -> Self;

    fn uniform(value: f32) -> Self;

    fn scaled(&self, scaling: impl ScalingTrait) -> Self;
}

impl<T> CornerRadiusTraitExt for T
where
    T: CornerRadiusTrait,
{
    fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }

    fn uniform(value: f32) -> Self {
        Self::new(value, value, value, value)
    }

    fn scaled(&self, scaling: impl ScalingTrait) -> Self {
        let factor = scaling.get_factor();
        Self::new(
            self.top_left() * factor,
            self.top_right() * factor,
            self.bottom_right() * factor,
            self.bottom_left() * factor,
        )
    }
}

// TODO: Rename
pub trait PositionTrait: Copy {
    fn new(left: f32, top: f32) -> Self;
    fn left(&self) -> f32;
    fn top(&self) -> f32;
}

pub trait PositionTraitExt {
    fn zero() -> Self;

    fn only_left(left: f32) -> Self;

    fn only_top(top: f32) -> Self;

    fn from_size(size: impl SizeTrait) -> Self;

    fn offset(&self, size: impl SizeTrait) -> Self;

    fn combined(&self, other: Self) -> Self;

    fn remaining<Size>(&self, size: Size) -> Size
    where
        Size: SizeTrait;

    fn relative_to(&self, other: Self) -> Self;

    fn scaled(&self, scaling: impl ScalingTrait) -> Self;

    fn is_equal(&self, rhs: Self) -> bool;
}

impl<T> PositionTraitExt for T
where
    T: PositionTrait,
{
    fn zero() -> Self {
        Self::new(0.0, 0.0)
    }

    fn only_left(left: f32) -> Self {
        Self::new(left, 0.0)
    }

    fn only_top(top: f32) -> Self {
        Self::new(0.0, top)
    }

    fn from_size(size: impl SizeTrait) -> Self {
        Self::new(size.width(), size.height())
    }

    fn offset(&self, size: impl SizeTrait) -> Self {
        Self::new(self.left() + size.width(), self.top() + size.height())
    }

    fn combined(&self, other: Self) -> Self {
        Self::new(self.left() + other.left(), self.top() + other.top())
    }

    // TODO: Rename this given how it's used
    fn remaining<Size>(&self, size: Size) -> Size
    where
        Size: SizeTrait,
    {
        Size::new(self.left() + size.width(), self.top() + size.height())
    }

    fn relative_to(&self, other: Self) -> Self {
        Self::new(self.left() - other.left(), self.top() - other.top())
    }

    fn scaled(&self, scaling: impl ScalingTrait) -> Self {
        let factor = scaling.get_factor();
        Self::new(self.left() * factor, self.top() * factor)
    }

    fn is_equal(&self, rhs: Self) -> bool {
        self.left() == rhs.left() && self.top() == rhs.top()
    }
}

pub trait SizeTrait: Copy {
    fn new(width: f32, height: f32) -> Self;
    fn width(&self) -> f32;
    fn height(&self) -> f32;
}

pub trait SizeTraitExt {
    fn zero() -> Self;

    fn only_width(width: f32) -> Self;

    fn only_height(height: f32) -> Self;

    fn grow(&self, growth: Self) -> Self;

    fn shrink(&self, size: Self) -> Self;

    fn scaled(&self, scaling: impl ScalingTrait) -> Self;

    fn halved(&self) -> Self;

    fn doubled(&self) -> Self;

    fn is_equal(&self, rhs: Self) -> bool;
}

impl<T> SizeTraitExt for T
where
    T: SizeTrait,
{
    fn zero() -> Self {
        Self::new(0.0, 0.0)
    }

    fn only_width(width: f32) -> Self {
        Self::new(width, 0.0)
    }

    fn only_height(height: f32) -> Self {
        Self::new(0.0, height)
    }

    fn grow(&self, size: Self) -> Self {
        Self::new(self.width() + size.width(), self.height() + size.height())
    }

    fn shrink(&self, size: Self) -> Self {
        Self::new(self.width() - size.width(), self.height() - size.height())
    }

    fn scaled(&self, scaling: impl ScalingTrait) -> Self {
        let factor = scaling.get_factor();
        Self::new(self.width() * factor, self.height() * factor)
    }

    fn halved(&self) -> Self {
        Self::new(self.width() / 2.0, self.height() / 2.0)
    }

    fn doubled(&self) -> Self {
        Self::new(self.width() * 2.0, self.height() * 2.0)
    }

    fn is_equal(&self, rhs: Self) -> bool {
        self.width() == rhs.width() && self.height() == rhs.height()
    }
}

pub trait PartialSizeTrait: Copy {
    fn new(width: f32, height: Option<f32>) -> Self;
    fn width(&self) -> f32;
    fn height(&self) -> Option<f32>;
}

pub trait PartialSizeTraitExt {
    fn finalize<Size>(self) -> Size
    where
        Size: SizeTrait;

    fn finalize_or<Size>(self, height: f32) -> Size
    where
        Size: SizeTrait;
}

impl<T> PartialSizeTraitExt for T
where
    T: PartialSizeTrait,
{
    fn finalize<Size>(self) -> Size
    where
        Size: SizeTrait,
    {
        let width = self.width();
        let height = self.height().expect("element cannot have flexible height");

        Size::new(width, height)
    }

    fn finalize_or<Size>(self, height: f32) -> Size
    where
        Size: SizeTrait,
    {
        let width = self.width();
        let height = self.height().unwrap_or(height);

        Size::new(width, height)
    }
}

pub trait ClipTrait: Copy {
    fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self;
    fn left(&self) -> f32;
    fn right(&self) -> f32;
    fn top(&self) -> f32;
    fn bottom(&self) -> f32;
}

pub trait WindowCache<App>
where
    App: Application,
{
    fn create() -> Self;

    fn register_window(&mut self, window_class: &str, position: App::Position, size: App::Size);

    fn update_position(&mut self, window_class: &str, position: App::Position);

    fn update_size(&mut self, window_class: &str, size: App::Size);

    fn get_window_state(&self, window_class: &str) -> Option<(App::Position, App::Size)>;
}

pub struct FocusState<App>
where
    App: Application,
{
    focused_element: Option<WeakElementCell<App>>,
    focused_window: Option<usize>,
    previous_hovered_element: Option<WeakElementCell<App>>,
    previous_hovered_window: Option<usize>,
    previous_focused_element: Option<WeakElementCell<App>>,
    previous_focused_window: Option<usize>,
}

impl<App> Default for FocusState<App>
where
    App: Application,
{
    fn default() -> Self {
        Self {
            focused_element: Default::default(),
            focused_window: Default::default(),
            previous_hovered_element: Default::default(),
            previous_hovered_window: Default::default(),
            previous_focused_element: Default::default(),
            previous_focused_window: Default::default(),
        }
    }
}

impl<App> FocusState<App>
where
    App: Application,
{
    pub fn remove_focus(&mut self) {
        self.focused_element = None;
        self.focused_window = None;
    }

    pub fn set_focused_element(&mut self, element: Option<ElementCell<App>>, window_index: usize) {
        self.focused_element = element.as_ref().map(Rc::downgrade);
        self.focused_window = Some(window_index);
    }

    pub fn set_focused_window(&mut self, window_index: usize) {
        self.focused_window = Some(window_index);
    }

    pub fn get_focused_window(&self) -> Option<usize> {
        self.focused_window
    }

    pub fn update_focused_element(&mut self, element: Option<ElementCell<App>>, window_index: usize) {
        if let Some(element) = element {
            self.focused_element = Some(Rc::downgrade(&element));
            self.focused_window = Some(window_index);
        }
    }

    pub fn get_focused_element(&self) -> Option<(ElementCell<App>, usize)> {
        let element = self.focused_element.clone();
        element.as_ref().and_then(Weak::upgrade).zip(self.focused_window)
    }

    pub fn did_hovered_element_change(&self, hovered_element: &Option<ElementCell<App>>) -> bool {
        self.previous_hovered_element
            .as_ref()
            .zip(hovered_element.as_ref())
            .map(|(previous, current)| !Weak::ptr_eq(previous, &Rc::downgrade(current)))
            .unwrap_or(self.previous_hovered_element.is_some() || hovered_element.is_some())
    }

    pub fn did_focused_element_change(&self) -> bool {
        self.previous_focused_element
            .as_ref()
            .zip(self.focused_element.as_ref())
            .map(|(previous, current)| !Weak::ptr_eq(previous, current))
            .unwrap_or(self.previous_focused_element.is_some() || self.focused_element.is_some())
    }

    pub fn previous_hovered_window(&self) -> Option<usize> {
        self.previous_hovered_window
    }

    pub fn focused_window(&self) -> Option<usize> {
        self.focused_window
    }

    pub fn previous_focused_window(&self) -> Option<usize> {
        self.previous_focused_window
    }

    pub fn update(&mut self, hovered_element: &Option<ElementCell<App>>, window_index: Option<usize>) -> Option<ElementCell<App>> {
        self.previous_hovered_element = hovered_element.as_ref().map(Rc::downgrade);
        self.previous_hovered_window = window_index;

        self.previous_focused_element = self.focused_element.clone();
        self.previous_focused_window = self.focused_window;

        self.focused_element.clone().and_then(|weak_element| weak_element.upgrade())
    }
}
