use std::any::{Any, TypeId};

use rust_state::RustState;

use crate::application::Application;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TooltipId(TypeId);

pub trait TooltipExt {
    fn tooltip_id(&self) -> TooltipId;
}

impl<T> TooltipExt for T
where
    T: Any,
{
    fn tooltip_id(&self) -> TooltipId {
        TooltipId(T::type_id(self))
    }
}

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TooltipTheme<App>
where
    App: Application + 'static,
{
    pub background_color: App::Color,
    pub foreground_color: App::Color,
    pub highlight_color: App::Color,
    pub shadow_color: App::Color,
    pub shadow_padding: App::ShadowPadding,
    pub font_size: App::FontSize,
    pub overflow_behavior: App::OverflowBehavior,
    pub corner_diameter: App::CornerDiameter,
    pub border: f32,
    pub gap: f32,
    pub mouse_offset: f32,
}

pub struct Tooltip<'a> {
    pub text: &'a str,
    pub id: TooltipId,
}
