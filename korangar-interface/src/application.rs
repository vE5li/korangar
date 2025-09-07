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

    /// Application corner diameter type.
    ///
    /// Ideally this should be the same type that the application renderer uses
    /// to represent corner diameter.
    type CornerDiameter: CornerDiameter;

    /// Application font size type.
    ///
    /// Ideally this should be the same type that the application renderer uses
    /// to represent font size.
    type FontSize: FontSize;

    /// Application 2D position type.
    ///
    /// Ideally this should be the same type that the application renderer uses
    /// to represent 2D positions. This might not be possible without conversion
    /// if the application doesn't represent positions from the top left of
    /// the screen.
    type Position: Position;

    /// Application 2D size type.
    ///
    /// Ideally this should be the same type that the application renderer uses
    /// to represent 2D sizes.
    type Size: Size;

    /// Application clip type.
    ///
    /// The clip is used to avoid rendering outside the bounds of a parent
    /// element or window.
    ///
    /// Ideally this should be the same type that the application renderer uses
    /// to represent clips.
    type Clip: Clip;

    /// Application shadow padding.
    ///
    /// Defines the padding of rectangle shadows in pixels.
    ///
    /// Ideally this should be the same type that the application renderer uses
    /// to represent shadows.
    type ShadowPadding: ShadowPadding;

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

    /// Application behavior when text overflows.
    ///
    /// Typically this would include options line inserting a line break and
    /// shrinking the text.
    type OverflowBehavior: Copy;

    /// Application text layouter.
    type TextLayouter: TextLayouter<Self>;

    fn set_current_theme_type(theme: Self::ThemeType);
}

/// A type for saving and loading window positions and sizes.
///
/// [`korangar_interface`] does not depend on any specific behavior of the
/// window cache, therefore the actual logic of this is up to the implementer.
/// The implementer may choose to only store the window data in RAM, to save it
/// to a file, or to add extra logic for the position and size it returns.
///
/// All the operations on the cache require the window to have a
/// [`WindowClass`](Application::WindowClass`), otherwise there is no way to
/// identify them and they will not be cached.
pub trait WindowCache<App>
where
    App: Application,
{
    /// Create or load a new instance of the window cache.
    fn create() -> Self;

    /// Attempt to get the position and size for a given window class.
    fn get_window_state(&self, window_class: App::WindowClass) -> Option<(Anchor<App>, App::Size)>;

    /// Register a new window with its size and position.
    ///
    /// This is only called if the window is not already cached but the
    /// implementer should not rely on that.
    fn register_window(&mut self, window_class: App::WindowClass, anchor: Anchor<App>, size: App::Size);

    /// Update the anchor of a registered window.
    fn update_anchor(&mut self, window_class: App::WindowClass, anchor: Anchor<App>);

    /// Update the size of a registered window.
    fn update_size(&mut self, window_class: App::WindowClass, size: App::Size);
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
    #[allow(clippy::too_many_arguments)]
    fn render_rectangle(
        &self,
        position: App::Position,
        size: App::Size,
        clip: App::Clip,
        corner_diameter: App::CornerDiameter,
        color: App::Color,
        shadow_color: App::Color,
        shadow_padding: App::ShadowPadding,
    );

    /// Render a str as text.
    #[allow(clippy::too_many_arguments)]
    fn render_text(
        &self,
        text: &str,
        position: App::Position,
        available_width: f32,
        clip: App::Clip,
        color: App::Color,
        highlight_color: App::Color,
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
    /// Calculate the size of a given string. Depending on the overflow
    /// behavior, the text might shrink, so we also return a new font size.
    fn get_text_dimensions(
        &self,
        text: &str,
        default_color: App::Color,
        highlight_color: App::Color,
        font_size: App::FontSize,
        available_width: f32,
        overflow_behavior: App::OverflowBehavior,
    ) -> (App::Size, App::FontSize);
}

/// Size for text elements.
pub trait FontSize: Copy {
    /// Scale the font size.
    fn scaled(&self, scaling: f32) -> Self;
}

/// 2D position.
pub trait Position: Copy {
    /// Create new position from the left and top screen offset in pixels.
    fn new(left: f32, top: f32) -> Self;

    /// Get position from the left of the screen in pixels.
    fn left(&self) -> f32;

    /// Get position from the top of the screen in pixels.
    fn top(&self) -> f32;
}

/// 2D size.
pub trait Size: Copy {
    /// Create a new size from the width and height in pixels.
    fn new(width: f32, height: f32) -> Self;

    /// Get the width in pixels.
    fn width(&self) -> f32;

    /// Get the height in pixels.
    fn height(&self) -> f32;
}

/// Rectangle corner diameter.
pub trait CornerDiameter: Copy {
    /// Create new corner diameter from the corner radii in pixels.
    fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self;

    /// Scale the corner diameter in pixels.
    fn scaled(&self, scaling: f32) -> Self;

    /// Get the top left corner diameter in pixels.
    fn top_left(&self) -> f32;

    /// Get the top right corner diameter in pixels.
    fn top_right(&self) -> f32;

    /// Get the bottom right corner diameter in pixels.
    fn bottom_right(&self) -> f32;

    /// Get the bottom left corner diameter in pixels.
    fn bottom_left(&self) -> f32;
}

/// A clip for masking rendering operations. The clip is defined by its four
/// bounds (left, right, top, and bottom) and only pixels within this bound will
/// be rendered.
///
/// This is important for components such as
/// [`scroll_view`](crate::components::scroll_view)s.
pub trait Clip: Copy {
    /// Create a new clip from the boundaries in pixels.
    fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self;

    /// Create a new unbound clip.
    fn unbound() -> Self;

    /// Get the left boundary. No pixel with an x coordinate smaller that this
    /// will be rendered.
    fn left(&self) -> f32;

    /// Get the right boundary. No pixel with an x coordinate larger that this
    /// will be rendered.
    fn right(&self) -> f32;

    /// Get the top boundary. No pixel with a y coordinate smaller that this
    /// will be rendered.
    fn top(&self) -> f32;

    /// Get the bottom boundary. No pixel with a y coordinate larger that this
    /// will be rendered.
    fn bottom(&self) -> f32;
}

/// The amount of shadows added to the sides of the rectangle.
pub trait ShadowPadding: Copy {
    /// No shadows at all.
    fn none() -> Self;

    /// Scale the shadows.
    fn scaled(&self, scaling: f32) -> Self;
}
