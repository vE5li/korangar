const fn symbol(enabled: &'static str, disabled: &'static str) -> &'static str {
    match cfg!(feature = "unicode") {
        true => enabled,
        false => disabled,
    }
}

pub const NEWLINE: &str = symbol("⮎", ">");
pub const ARROW: &str = symbol("→", "->");
