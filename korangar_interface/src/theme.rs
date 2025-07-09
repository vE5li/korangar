use rust_state::Path;

use crate::application::Application;
use crate::components::button::ButtonTheme;
use crate::components::collapsable::CollapsableTheme;
use crate::components::drop_down::DropDownTheme;
use crate::components::state_button::StateButtonTheme;
use crate::components::text::TextTheme;
use crate::components::text_box::TextBoxTheme;
use crate::window::WindowTheme;

pub trait ThemePathGetter<App: Application>: Copy {
    fn new() -> Self;

    fn window(self) -> impl Path<App, WindowTheme<App>>;

    fn text(self) -> impl Path<App, TextTheme<App>>;

    fn button(self) -> impl Path<App, ButtonTheme<App>>;

    fn state_button(self) -> impl Path<App, StateButtonTheme<App>>;

    fn text_box(self) -> impl Path<App, TextBoxTheme<App>>;

    fn collapsable(self) -> impl Path<App, CollapsableTheme<App>>;

    fn drop_down(self) -> impl Path<App, DropDownTheme<App>>;
}

pub fn theme<App: Application>() -> impl ThemePathGetter<App> {
    App::ThemeGetter::new()
}

// TODO: Rename `theme` to `theme_internal` or something and expose theme like
// this. We want to do that be cause impl ThemePathGetter will not allow the
// end user to get custom themes but without it this crate is unable to
// infer the types. pub fn theme<App: Application>() -> App::ThemeGetter {
//     App::ThemeGetter::new()
// }
