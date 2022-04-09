pub fn newline_symbol() -> &'static str {

    #[cfg(feature = "unicode")]
    return "⮎";

    #[cfg(not(feature = "unicode"))]
    return ">";
}

pub fn arrow_symbol() -> &'static str {

    #[cfg(feature = "unicode")]
    return "→";

    #[cfg(not(feature = "unicode"))]
    return "->";
}
