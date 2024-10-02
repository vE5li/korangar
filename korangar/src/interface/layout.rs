use cgmath::Vector2;
use derive_new::new;
use korangar_interface::elements::ElementDisplay;
use serde::{Deserialize, Serialize};
use winit::dpi::PhysicalSize;

pub trait ArrayType {
    type Element;

    const ELEMENT_COUNT: usize;

    fn get_array_fields(&'static self) -> [(String, &'static Self::Element); Self::ELEMENT_COUNT];

    fn get_inner(&self) -> [Self::Element; Self::ELEMENT_COUNT];
}

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
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
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
    fn display(&self) -> String {
        format!("^FFBB00↦^000000{} ^FFBB00↧^000000{}", self.left.display(), self.top.display())
    }
}

impl ArrayType for ScreenPosition {
    type Element = f32;

    const ELEMENT_COUNT: usize = 2;

    fn get_array_fields(&'static self) -> [(String, &'static Self::Element); Self::ELEMENT_COUNT] {
        [("left".to_owned(), &self.left), ("top".to_owned(), &self.top)]
    }

    fn get_inner(&self) -> [Self::Element; Self::ELEMENT_COUNT] {
        [self.left, self.top]
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
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
    fn display(&self) -> String {
        format!(
            "^FFBB00↔^000000{} ^FFBB00↕^000000{}",
            self.width.display(),
            self.height.display(),
        )
    }
}

impl ArrayType for ScreenSize {
    type Element = f32;

    const ELEMENT_COUNT: usize = 2;

    fn get_array_fields(&'static self) -> [(String, &'static Self::Element); Self::ELEMENT_COUNT] {
        [("width".to_owned(), &self.width), ("height".to_owned(), &self.height)]
    }

    fn get_inner(&self) -> [Self::Element; Self::ELEMENT_COUNT] {
        [self.width, self.height]
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

impl From<ScreenClip> for [f32; 4] {
    fn from(val: ScreenClip) -> Self {
        [val.left, val.top, val.right, val.bottom]
    }
}

#[derive(Debug, Copy, Clone, new)]
pub struct PartialScreenSize {
    pub width: f32,
    pub height: Option<f32>,
}

impl PartialScreenSize {
    pub fn finalize(self) -> ScreenSize {
        let width = self.width;
        let height = self.height.expect("element cannot have flexible height");

        ScreenSize { width, height }
    }

    pub fn finalize_or(self, height: f32) -> ScreenSize {
        let width = self.width;
        let height = self.height.unwrap_or(height);

        ScreenSize { width, height }
    }
}

impl From<ScreenSize> for PartialScreenSize {
    fn from(size: ScreenSize) -> Self {
        Self {
            width: size.width,
            height: Some(size.height),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CornerRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_left: f32,
    pub bottom_right: f32,
}

impl CornerRadius {
    pub fn uniform(value: f32) -> Self {
        Self {
            top_left: value,
            top_right: value,
            bottom_left: value,
            bottom_right: value,
        }
    }
}

impl ElementDisplay for CornerRadius {
    fn display(&self) -> String {
        format!(
            "^FFBB00↖^000000{} ^FFBB00↗^000000{} ^FFBB00↘^000000{} ^FFBB00↙^000000{}",
            self.top_left.display(),
            self.top_right.display(),
            self.bottom_right.display(),
            self.bottom_left.display()
        )
    }
}

impl ArrayType for CornerRadius {
    type Element = f32;

    const ELEMENT_COUNT: usize = 4;

    fn get_array_fields(&'static self) -> [(String, &'static Self::Element); Self::ELEMENT_COUNT] {
        [
            ("top left".to_owned(), &self.top_left),
            ("top right".to_owned(), &self.top_right),
            ("bottom right".to_owned(), &self.bottom_right),
            ("bottom left".to_owned(), &self.bottom_left),
        ]
    }

    fn get_inner(&self) -> [Self::Element; Self::ELEMENT_COUNT] {
        [self.top_left, self.top_right, self.bottom_right, self.bottom_left]
    }
}

impl std::ops::Mul<f32> for CornerRadius {
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

impl From<CornerRadius> for [f32; 4] {
    fn from(val: CornerRadius) -> Self {
        [val.top_left, val.top_right, val.bottom_right, val.bottom_left]
    }
}
