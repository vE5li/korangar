const fn color(enabled: &'static str) -> &'static str {
    match cfg!(feature = "colors") {
        true => enabled,
        false => "",
    }
}

pub const GREEN: &'static str = color("\x1B[32m");
pub const RED: &'static str = color("\x1B[31m");
pub const CYAN: &'static str = color("\x1B[36m");
pub const YELLOW: &'static str = color("\x1B[33m");
pub const MAGENTA: &'static str = color("\x1B[35m");
pub const NONE: &'static str = color("\x1B[0m");
