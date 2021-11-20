mod text;
mod rectangle;
mod stretch;
mod checkbox;
mod hoverable;
mod clickable;
mod draggable;
mod container;

pub use self::text::{ TextComponent, DynamicTextComponent };
pub use self::rectangle::RectangleComponent;
pub use self::stretch::StretchComponent;
pub use self::checkbox::CheckboxComponent;
pub use self::hoverable::HoverableComponent;
pub use self::clickable::ClickableComponent;
pub use self::draggable::DraggableComponent;
pub use self::container::ContainerComponent;

pub enum Component {
    Text(TextComponent),
    DynamicText(DynamicTextComponent),
    Rectangle(RectangleComponent),
    Stretch(StretchComponent),
    Checkbox(CheckboxComponent),
    Hoverable(HoverableComponent),
    Clickable(ClickableComponent),
    Draggable(DraggableComponent),
    Container(ContainerComponent),
}
