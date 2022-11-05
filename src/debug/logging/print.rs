use super::*;

macro_rules! print_debug {
    ($format:expr) => (print_indented(String::from($format), true));
    ($format:expr, $($arguments:tt)*) => (print_indented(format!($format, $($arguments)*), true));
}

macro_rules! print_debug_prefix {
    ($format:expr) => (print_indented(String::from($format), false));
    ($format:expr, $($arguments:tt)*) => (print_indented(format!($format, $($arguments)*), false));
}

pub fn print_indented(message: String, newline: bool) {
    let offset = message_offset();

    if stack_size() > 0 {
        if get_message_count() == 0 {
            println!(" {} started", ARROW);
        }

        increment_message_count();
    }

    for _ in 0..offset {
        print!(" ");
    }

    if offset != 0 {
        print!("{} ", NEWLINE);
    }

    print!("{}", message);

    if newline {
        println!();
    }
}

pub fn vulkan_message_callback(message: &vulkano::instance::debug::Message) {
    let severity = if message.severity.error {
        "error"
    } else if message.severity.warning {
        "warning"
    } else if message.severity.information {
        "information"
    } else if message.severity.verbose {
        "verbose"
    } else {
        panic!("no-impl");
    };

    let message_type = if message.ty.general {
        "general"
    } else if message.ty.validation {
        "validation"
    } else if message.ty.performance {
        "performance"
    } else {
        panic!("no-impl");
    };

    print_debug!(
        "{}{}{} [{}{}{}] [{}{}{}]: {}",
        MAGENTA,
        message.layer_prefix.unwrap_or("unknown"),
        NONE,
        YELLOW,
        message_type,
        NONE,
        RED,
        severity,
        NONE,
        message.description
    );
}
