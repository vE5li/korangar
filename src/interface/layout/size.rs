use derive_new::new;
use serde::{Deserialize, Serialize};

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

    pub fn only_top(top: f32) -> Self {
        Self { left: 0.0, top }
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

impl ScreenSize {
    pub fn only_width(width: f32) -> Self {
        Self { width, height: 0.0 }
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

#[derive(Copy, Clone, new)]
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

// TODO: Temorary, remove at some point
impl From<cgmath::Vector4<f32>> for CornerRadius {
    fn from(val: cgmath::Vector4<f32>) -> Self {
        Self {
            top_left: val.x,
            top_right: val.y,
            bottom_right: val.z,
            bottom_left: val.w,
        }
    }
}
