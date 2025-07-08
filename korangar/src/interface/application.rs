use std::marker::ConstParamTy;
use std::sync::Arc;

#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::application::ScalingTrait;
use korangar_interface::element::PrototypeElement;
use korangar_interface::event::ClickAction;
use korangar_interface::window::PrototypeWindow;
use ron::ser::PrettyConfig;
use rust_state::Path;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use super::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};
use super::resource::{Move, PartialMove};
use super::theme::GameTheme;
use super::windows::WindowCache;
use crate::graphics::Color;
use crate::input::{MouseInputMode, UserEvent};
use crate::loaders::{FontLoader, FontSize, Scaling};
use crate::renderer::InterfaceRenderer;

const DEFAULT_FONTS: &[&str] = &["NotoSans", "NotoSansKR"];

impl korangar_interface::application::SizeTrait for ScreenSize {
    fn new(width: f32, height: f32) -> Self {
        ScreenSize { width, height }
    }

    fn width(&self) -> f32 {
        self.width
    }

    fn height(&self) -> f32 {
        self.height
    }
}

impl korangar_interface::application::PositionTrait for ScreenPosition {
    fn new(left: f32, top: f32) -> Self {
        ScreenPosition { left, top }
    }

    fn left(&self) -> f32 {
        self.left
    }

    fn top(&self) -> f32 {
        self.top
    }
}

impl korangar_interface::application::ClipTrait for ScreenClip {
    fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self { left, right, top, bottom }
    }

    fn unbound() -> Self {
        Self::new(0.0, 0.0, f32::MAX, f32::MAX)
    }

    fn left(&self) -> f32 {
        self.left
    }

    fn right(&self) -> f32 {
        self.right
    }

    fn top(&self) -> f32 {
        self.top
    }

    fn bottom(&self) -> f32 {
        self.bottom
    }
}

impl korangar_interface::application::CornerRadiusTrait for CornerRadius {
    fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        Self {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        }
    }

    fn top_left(&self) -> f32 {
        self.top_left
    }

    fn top_right(&self) -> f32 {
        self.top_right
    }

    fn bottom_right(&self) -> f32 {
        self.bottom_right
    }

    fn bottom_left(&self) -> f32 {
        self.bottom_left
    }
}
