#[cfg(feature = "plain")]
mod color_codes {
    pub const GREEN: &str = "";
    pub const RED: &str = "";
    pub const CYAN: &str = "";
    pub const YELLOW: &str = "";
    pub const MAGENTA: &str = "";
    pub const NONE: &str = "";
}

#[cfg(not(feature = "plain"))]
mod color_codes {
    pub const GREEN: &str = "\x1B[32m";
    pub const RED: &str = "\x1B[31m";
    pub const CYAN: &str = "\x1B[36m";
    pub const YELLOW: &str = "\x1B[33m";
    pub const MAGENTA: &str = "\x1B[35m";
    pub const NONE: &str = "\x1B[0m";
}

use std::fmt::{Debug, Display};

use self::color_codes::*;

/// Wrapper struct to print anything that implements [`Display`] or [`Debug`].
pub struct Colorized<'a, T> {
    wrapped: &'a T,
    color: &'static str,
}

impl<'a, T> Display for Colorized<'a, T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", self.color, self.wrapped, NONE)
    }
}

impl<'a, T> Debug for Colorized<'a, T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{:?}{}", self.color, self.wrapped, NONE)
    }
}

/// Wrap any type in a [`Colorized`].
pub trait Colorize
where
    Self: Sized,
{
    fn green(&self) -> Colorized<'_, Self> {
        Colorized {
            wrapped: self,
            color: GREEN,
        }
    }

    fn red(&self) -> Colorized<'_, Self> {
        Colorized { wrapped: self, color: RED }
    }

    fn cyan(&self) -> Colorized<'_, Self> {
        Colorized {
            wrapped: self,
            color: CYAN,
        }
    }

    fn yellow(&self) -> Colorized<'_, Self> {
        Colorized {
            wrapped: self,
            color: YELLOW,
        }
    }

    fn magenta(&self) -> Colorized<'_, Self> {
        Colorized {
            wrapped: self,
            color: MAGENTA,
        }
    }
}

impl<T> Colorize for T {}
