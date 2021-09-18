pub fn green() -> &'static str {

    #[cfg(feature = "colors")]
    return "\x1B[32m";

    #[cfg(not(feature = "colors"))]
    return "";
}

pub fn red() -> &'static str {

    #[cfg(feature = "colors")]
    return "\x1B[31m";

    #[cfg(not(feature = "colors"))]
    return "";
}

pub fn cyan() -> &'static str {

    #[cfg(feature = "colors")]
    return "\x1B[36m";

    #[cfg(not(feature = "colors"))]
    return "";
}

pub fn yellow() -> &'static str {

    #[cfg(feature = "colors")]
    return "\x1B[33m";

    #[cfg(not(feature = "colors"))]
    return "";
}

pub fn magenta() -> &'static str {

    #[cfg(feature = "colors")]
    return "\x1B[35m";

    #[cfg(not(feature = "colors"))]
    return "";
}

pub fn none() -> &'static str {

    #[cfg(feature = "colors")]
    return "\x1B[0m";

    #[cfg(not(feature = "colors"))]
    return "";
}
