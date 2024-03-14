use crate::application::Application;
use crate::elements::ElementCell;

pub enum HoverInformation<App>
where
    App: Application,
{
    Element(ElementCell<App>),
    Hovered,
    Missed,
}
