use rust_state::Path;

use crate::application::Application;
use crate::components::button::ButtonTheme;
use crate::components::collapsible::CollapsibleTheme;
use crate::components::drop_down::DropDownTheme;
use crate::components::field::FieldTheme;
use crate::components::state_button::StateButtonTheme;
use crate::components::text::TextTheme;
use crate::components::text_box::TextBoxTheme;
use crate::layout::tooltip::TooltipTheme;
use crate::window::WindowTheme;

/// Glue between [`korangar_interface`] and the final application. This trait
/// allows the final application to define the theme with any layout and with
/// additional fields, so long is it can return paths to all of the themes this
/// crate needs to render the basic components.
pub trait ThemePathGetter<App: Application>: Copy {
    /// Create a new path. This is only used in [`theme`].
    fn new() -> Self;

    /// Path to the window theme.
    fn window(self) -> impl Path<App, WindowTheme<App>>;

    /// Path to the text theme.
    fn text(self) -> impl Path<App, TextTheme<App>>;

    /// Path to the button theme.
    fn button(self) -> impl Path<App, ButtonTheme<App>>;

    /// Path to the state button theme.
    fn state_button(self) -> impl Path<App, StateButtonTheme<App>>;

    /// Path to the text box theme.
    fn text_box(self) -> impl Path<App, TextBoxTheme<App>>;

    /// Path to the collapsible theme.
    fn collapsible(self) -> impl Path<App, CollapsibleTheme<App>>;

    /// Path to the drop down theme.
    fn drop_down(self) -> impl Path<App, DropDownTheme<App>>;

    /// Path to the field theme.
    fn field(self) -> impl Path<App, FieldTheme<App>>;

    /// Path to the tooltip theme.
    fn tooltip(self) -> impl Path<App, TooltipTheme<App>>;
}

/// Path to the theme of the window.
pub fn theme<App: Application>() -> impl ThemePathGetter<App> {
    App::ThemeGetter::new()
}
