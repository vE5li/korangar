use crate::layout::alignment::OverflowBehavior;
use crate::layout::{ClipLayer, Icon};
use crate::theme::ThemePathGetter;
use crate::window::Anchor;

/// Glue between [`korangar_interface`] and the application.
///
/// This trait mostly consists of associated types so the application can call
/// and integrate with this crate using its own types. This was a design
/// decision to reduce the number of types and type conversion in the
/// application but it makes most types in this crate generic over
/// a `App: Application`, which makes code inside this crate harder to write. I
/// feel this is a worthwhile trade-off but I might revisit this in the future.
pub trait Application: Sized + 'static {
    /// Window cache used to store window position and size.
    ///
    /// This can be implemented in a volatile manner so window positions are
    /// only saved until the application is close or in a persistent manner
    /// where the cache is saved to and loaded from a file.
    type Cache: WindowCache<Self>;

    /// Type to specify the theme when creating a window.
    ///
    /// Typically this would be an enum with each variant being one of the
    /// application themes but can also be a unit type if the application
    /// only has a single theme.
    ///
    /// The [`Default`] bound is used when deriving
    /// [`StateWindow`](crate::window::StateWindow).
    type ThemeType: Default + Copy;

    /// Glue between [`korangar_interface`] and the final application theme. See
    /// [`ThemePathGetter`] for more information.
    type ThemeGetter: ThemePathGetter<Self>;

    /// Application color type.
    ///
    /// Ideally this should be the same type that the application renderer uses
    /// to represent color.
    type Color: Copy;

    /// Application corner radius type.
    ///
    /// Ideally this should be the same type that the application renderer uses
    /// to represent corner radius.
    type CornerRadius: CornerRadiusTrait;

    /// Application font size type.
    ///
    /// Ideally this should be the same type that the application renderer uses
    /// to represent font size.
    type FontSize: FontSizeTrait;

    /// Application 2D position type.
    ///
    /// Ideally this should be the same type that the application renderer uses
    /// to represent 2D positions. This might not be possible without conversion
    /// if the application doesn't represent positions from the top left of
    /// the screen.
    type Position: PositionTrait;

    /// Application 2D size type.
    ///
    /// Ideally this should be the same type that the application renderer uses
    /// to represent 2D sizes.
    type Size: SizeTrait;

    /// Application clip type.
    ///
    /// The clip is used to avoid rendering outside the bounds of a parent
    /// element or window.
    ///
    /// Ideally this should be the same type that the application renderer uses
    /// to represent clips.
    type Clip: ClipTrait;

    /// Renderer of the application.
    type Renderer: RenderLayer<Self>;

    /// Application window classes.
    ///
    /// The application can define classes for its windows to enable behaviour
    /// such as allowing only one window of a given class to be open at any
    /// point in time, restoring window position and size when the window is
    /// opened again (see [`WindowCache`], checking if a window with a given
    /// class is currently open, and closing windows with a given class.
    type WindowClass: PartialEq + Copy;

    /// Custom application event.
    ///
    /// When processing interface events (see
    /// [`process_events`](crate::Interface::process_events)) the interface will
    /// pass any custom events to the application.
    ///
    /// This allows components in the application to use the internal
    /// [`EventQueue`](crate::event::EventQueue) to register application
    /// specific events as well.
    type CustomEvent: Clone;

    /// Custom mouse mode.
    ///
    /// This allows the application to define additional mouse modes. E.g.
    /// dragging a resource or rotating the camera.
    type CustomMouseMode;

    /// Application text layouter.
    type TextLayouter: TextLayouter<Self>;

    fn set_current_theme_type(theme: Self::ThemeType);
}

/// Glue between [`korangar_interface`] and the renderer of the application.
pub trait RenderLayer<App: Application> {
    /// Application specific instruction.
    ///
    /// This allows extending the [`Layout`](crate::layout::Layout) to render
    /// application specific graphics.
    type CustomInstruction<'a>;

    /// Application specific icons.
    type CustomIcon: Clone + Copy;

    /// Render a rectangle.
    fn render_rectangle(
        &self,
        position: App::Position,
        size: App::Size,
        clip: App::Clip,
        corner_radius: App::CornerRadius,
        color: App::Color,
    );

    /// Render a str as text.
    fn render_text(
        &self,
        text: &str,
        position: App::Position,
        available_width: f32,
        clip: App::Clip,
        color: App::Color,
        font_size: App::FontSize,
    );

    /// Render an icon.
    fn render_icon(&self, position: App::Position, size: App::Size, clip: App::Clip, icon: Icon<App>, color: App::Color);

    /// Render a [`CustomInstruction`](RenderLayer::CustomInstruction).
    fn render_custom(&self, instruction: Self::CustomInstruction<'_>, clip_layers: &[ClipLayer<App>]);
}

/// Glue between [`korangar_interface`] and the part of the application that
/// handles layouting text. Many components change their size based on the
/// displayed text to avoid clipping text.
pub trait TextLayouter<App: Application>: Clone {
    fn get_text_dimensions(
        &self,
        text: &str,
        font_size: App::FontSize,
        available_width: f32,
        overflow_behavior: OverflowBehavior,
    ) -> (App::Size, App::FontSize);
}

// TODO: Rename
pub trait FontSizeTrait: Copy {
    fn scaled(&self, scaling: f32) -> Self;
}

// TODO: Rename
pub trait CornerRadiusTrait: Copy {
    fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self;

    fn scaled(&self, scaling: f32) -> Self;

    fn top_left(&self) -> f32;

    fn top_right(&self) -> f32;

    fn bottom_right(&self) -> f32;

    fn bottom_left(&self) -> f32;
}

// TODO: Rename
pub trait PositionTrait: Copy {
    fn new(left: f32, top: f32) -> Self;

    fn left(&self) -> f32;

    fn top(&self) -> f32;
}

// TODO: Rename
pub trait SizeTrait: Copy {
    fn new(width: f32, height: f32) -> Self;

    fn width(&self) -> f32;

    fn height(&self) -> f32;
}

// TODO: Rename
pub trait ClipTrait: Copy {
    fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self;

    fn unbound() -> Self;

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

    fn register_window(&mut self, window_class: App::WindowClass, anchor: Anchor<App>, size: App::Size);

    fn update_anchor(&mut self, window_class: App::WindowClass, anchor: Anchor<App>);

    fn update_size(&mut self, window_class: App::WindowClass, size: App::Size);

    fn get_window_state(&self, window_class: App::WindowClass) -> Option<(Anchor<App>, App::Size)>;
}
