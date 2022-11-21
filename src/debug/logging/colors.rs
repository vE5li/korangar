#[cfg(any(target_os = "windows", feature = "plain"))]
mod colors {
    pub const GREEN: &str = "";
    pub const RED: &str = "";
    pub const CYAN: &str = "";
    pub const YELLOW: &str = "";
    pub const MAGENTA: &str = "";
    pub const NONE: &str = "";
}

#[cfg(not(any(target_os = "windows", feature = "plain")))]
mod colors {
    pub const GREEN: &str = "\x1B[32m";
    pub const RED: &str = "\x1B[31m";
    pub const CYAN: &str = "\x1B[36m";
    pub const YELLOW: &str = "\x1B[33m";
    pub const MAGENTA: &str = "\x1B[35m";
    pub const NONE: &str = "\x1B[0m";
}

pub use self::colors::*;
