const fn color(enabled: &'static str) -> &'static str {
    match cfg!(feature = "colors") {
        true => enabled,
        false => "",
    }
}

pub const GREEN: &str = color("\x1B[32m");
pub const RED: &str = color("\x1B[31m");
pub const CYAN: &str = color("\x1B[36m");
pub const YELLOW: &str = color("\x1B[33m");
pub const MAGENTA: &str = color("\x1B[35m");
pub const NONE: &str = color("\x1B[0m");
