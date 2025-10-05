use cgmath::Vector2;
use korangar_interface::element::ElementDisplay;
use rust_state::RustState;
use serde::{Deserialize, Serialize};
use winit::dpi::PhysicalSize;

macro_rules! implement_ops {
    ($name:ident, $x:ident, $y:ident) => {
        impl $name {
            pub fn uniform(value: f32) -> Self {
                Self { $x: value, $y: value }
            }
        }

        impl std::ops::AddAssign<$name> for $name {
            fn add_assign(&mut self, rhs: Self) {
                self.$x += rhs.$x;
                self.$y += rhs.$y;
            }
        }

        impl std::ops::SubAssign<$name> for $name {
            fn sub_assign(&mut self, rhs: Self) {
                self.$x -= rhs.$x;
                self.$y -= rhs.$y;
            }
        }

        impl std::ops::Add<$name> for $name {
            type Output = Self;

            fn add(self, rhs: Self) -> Self {
                Self {
                    $x: self.$x + rhs.$x,
                    $y: self.$y + rhs.$y,
                }
            }
        }

        impl std::ops::Div<f32> for $name {
            type Output = Self;

            fn div(self, rhs: f32) -> Self {
                Self {
                    $x: self.$x / rhs,
                    $y: self.$y / rhs,
                }
            }
        }

        impl std::ops::Div<ScreenSize> for $name {
            type Output = Self;

            fn div(self, rhs: ScreenSize) -> Self {
                Self {
                    $x: self.$x / rhs.width,
                    $y: self.$y / rhs.height,
                }
            }
        }

        impl std::ops::Mul<f32> for $name {
            type Output = Self;

            fn mul(self, rhs: f32) -> Self {
                Self {
                    $x: self.$x * rhs,
                    $y: self.$y * rhs,
                }
            }
        }

        impl From<$name> for [f32; 2] {
            fn from(value: $name) -> Self {
                [value.$x, value.$y]
            }
        }
    };
}

/// The position as seen on screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, RustState)]
pub struct ScreenPosition {
    pub left: f32,
    pub top: f32,
}

impl ScreenPosition {
    pub fn from_size(ScreenSize { width, height }: ScreenSize) -> Self {
        Self { left: width, top: height }
    }

    pub fn only_left(left: f32) -> Self {
        Self { left, top: 0.0 }
    }

    pub fn only_top(top: f32) -> Self {
        Self { left: 0.0, top }
    }
}

impl ElementDisplay for ScreenPosition {
    fn element_display(&self) -> String {
        format!(
            "^000001→^000000{} ^000001↓^000000{}",
            // TODO: Use this instead. Not supported by the font currently.
            // "^000001↦^000000{} ^000001↧^000000{}",
            self.left.element_display(),
            self.top.element_display()
        )
    }
}

impl korangar_interface::application::Position for ScreenPosition {
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

implement_ops!(ScreenPosition, left, top);

impl std::ops::Sub<ScreenPosition> for ScreenPosition {
    type Output = ScreenSize;

    fn sub(self, rhs: ScreenPosition) -> ScreenSize {
        ScreenSize {
            width: self.left - rhs.left,
            height: self.top - rhs.top,
        }
    }
}

impl std::ops::AddAssign<ScreenSize> for ScreenPosition {
    fn add_assign(&mut self, rhs: ScreenSize) {
        self.left += rhs.width;
        self.top += rhs.height;
    }
}

/// The size as seen on screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, RustState)]
pub struct ScreenSize {
    pub width: f32,
    pub height: f32,
}

impl From<PhysicalSize<u32>> for ScreenSize {
    fn from(value: PhysicalSize<u32>) -> Self {
        Self {
            width: value.width as f32,
            height: value.height as f32,
        }
    }
}

impl From<Vector2<usize>> for ScreenSize {
    fn from(value: Vector2<usize>) -> Self {
        Self {
            width: value.x as f32,
            height: value.y as f32,
        }
    }
}

impl ScreenSize {
    pub fn only_width(width: f32) -> Self {
        Self { width, height: 0.0 }
    }
}

impl ElementDisplay for ScreenSize {
    fn element_display(&self) -> String {
        format!(
            "^000001↔^000000{} ^000001↕^000000{}",
            self.width.element_display(),
            self.height.element_display(),
        )
    }
}

impl korangar_interface::application::Size for ScreenSize {
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

implement_ops!(ScreenSize, width, height);

impl std::ops::Sub<ScreenSize> for ScreenSize {
    type Output = Self;

    fn sub(self, rhs: ScreenSize) -> Self {
        Self {
            width: self.width - rhs.width,
            height: self.height - rhs.height,
        }
    }
}

impl std::ops::Add<ScreenPosition> for ScreenSize {
    type Output = ScreenPosition;

    fn add(self, rhs: ScreenPosition) -> ScreenPosition {
        ScreenPosition {
            left: self.width + rhs.left,
            top: self.height + rhs.top,
        }
    }
}

impl std::ops::Add<ScreenSize> for ScreenPosition {
    type Output = ScreenPosition;

    fn add(self, rhs: ScreenSize) -> ScreenPosition {
        ScreenPosition {
            left: self.left + rhs.width,
            top: self.top + rhs.height,
        }
    }
}

impl std::ops::Sub<ScreenPosition> for ScreenSize {
    type Output = ScreenPosition;

    fn sub(self, rhs: ScreenPosition) -> ScreenPosition {
        ScreenPosition {
            left: self.width - rhs.left,
            top: self.height - rhs.top,
        }
    }
}

impl std::ops::Sub<ScreenSize> for ScreenPosition {
    type Output = ScreenPosition;

    fn sub(self, rhs: ScreenSize) -> ScreenPosition {
        ScreenPosition {
            left: self.left - rhs.width,
            top: self.top - rhs.height,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ScreenClip {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl ScreenClip {
    pub fn combine(&mut self, other: Self) {
        self.left = self.left.max(other.left);
        self.right = self.right.min(other.right);
        self.top = self.top.max(other.top);
        self.bottom = self.bottom.min(other.bottom);
    }
}

impl From<ScreenClip> for [f32; 4] {
    fn from(val: ScreenClip) -> Self {
        [val.left, val.top, val.right, val.bottom]
    }
}

impl korangar_interface::application::Clip for ScreenClip {
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

impl std::ops::Mul<f32> for ScreenClip {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        Self {
            left: self.left * rhs,
            right: self.right * rhs,
            bottom: self.bottom * rhs,
            top: self.top * rhs,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ShadowPadding {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl ShadowPadding {
    pub fn uniform(value: f32) -> Self {
        Self {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }

    pub fn diagonal(left_and_top: f32, right_and_bottom: f32) -> Self {
        Self {
            left: left_and_top,
            right: right_and_bottom,
            top: left_and_top,
            bottom: right_and_bottom,
        }
    }

    pub fn components(&self) -> [f32; 4] {
        [self.left, self.right, self.top, self.bottom]
    }
}

impl ElementDisplay for ShadowPadding {
    fn element_display(&self) -> String {
        format!(
            "^000001←^000000{} ^000001→^000000{} ^000001↑^000000{} ^000001↓^000000{}",
            self.left.element_display(),
            self.right.element_display(),
            self.top.element_display(),
            self.bottom.element_display()
        )
    }
}

impl korangar_interface::application::ShadowPadding for ShadowPadding {
    fn none() -> Self {
        Self::uniform(0.0)
    }

    fn scaled(&self, scaling: f32) -> Self {
        Self {
            left: self.left * scaling,
            right: self.right * scaling,
            top: self.top * scaling,
            bottom: self.bottom * scaling,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CornerDiameter {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_left: f32,
    pub bottom_right: f32,
}

impl CornerDiameter {
    pub fn uniform(value: f32) -> Self {
        Self {
            top_left: value,
            top_right: value,
            bottom_left: value,
            bottom_right: value,
        }
    }
}

impl ElementDisplay for CornerDiameter {
    fn element_display(&self) -> String {
        format!(
            "^000001↖^000000{} ^000001↗^000000{} ^000001↘^000000{} ^000001↙^000000{}",
            self.top_left.element_display(),
            self.top_right.element_display(),
            self.bottom_right.element_display(),
            self.bottom_left.element_display()
        )
    }
}

impl korangar_interface::application::CornerDiameter for CornerDiameter {
    fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        Self {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        }
    }

    fn scaled(&self, scaling: f32) -> Self {
        Self {
            top_left: self.top_left * scaling,
            top_right: self.top_right * scaling,
            bottom_left: self.bottom_left * scaling,
            bottom_right: self.bottom_right * scaling,
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

impl std::ops::Mul<f32> for CornerDiameter {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        Self {
            top_left: self.top_left * rhs,
            top_right: self.top_right * rhs,
            bottom_left: self.bottom_left * rhs,
            bottom_right: self.bottom_right * rhs,
        }
    }
}

impl From<CornerDiameter> for [f32; 4] {
    fn from(val: CornerDiameter) -> Self {
        [val.top_left, val.top_right, val.bottom_left, val.bottom_right]
    }
}
