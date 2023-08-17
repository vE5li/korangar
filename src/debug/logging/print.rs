use vulkano::instance::debug::{DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessengerCallbackData};

use super::*;

pub macro print_debug {
    ($format:expr) => (print_indented(String::from($format), true)),
    ($format:expr, $($arguments:tt)*) => (print_indented(format!($format, $($arguments)*), true)),
}

pub macro print_debug_prefix {
    ($format:expr) => (print_indented(String::from($format), false)),
    ($format:expr, $($arguments:tt)*) => (print_indented(format!($format, $($arguments)*), false)),
}

pub fn print_indented(message: String, newline: bool) {
    let offset = message_offset();

    if stack_size() > 0 {
        if get_message_count() == 0 {
            println!(" {ARROW} started");
        }

        increment_message_count();
    }

    for _ in 0..offset {
        print!(" ");
    }

    if offset != 0 {
        print!("{NEWLINE} ");
    }

    print!("{message}");

    if newline {
        println!();
    }
}

pub fn vulkan_message_callback(
    message_severity: DebugUtilsMessageSeverity,
    message_type: DebugUtilsMessageType,
    callback_data: DebugUtilsMessengerCallbackData<'_>,
) {
    let severity = if message_severity.intersects(DebugUtilsMessageSeverity::ERROR) {
        "error"
    } else if message_severity.intersects(DebugUtilsMessageSeverity::WARNING) {
        "warning"
    } else if message_severity.intersects(DebugUtilsMessageSeverity::INFO) {
        "information"
    } else if message_severity.intersects(DebugUtilsMessageSeverity::VERBOSE) {
        "verbose"
    } else {
        panic!("no-impl");
    };

    let message_type = if message_type.intersects(DebugUtilsMessageType::GENERAL) {
        "general"
    } else if message_type.intersects(DebugUtilsMessageType::VALIDATION) {
        "validation"
    } else if message_type.intersects(DebugUtilsMessageType::PERFORMANCE) {
        "performance"
    } else {
        panic!("no-impl");
    };

    print_debug!(
        "{}{}{} [{}{}{}] [{}{}{}]: {}",
        MAGENTA,
        callback_data.message_id_name.unwrap_or("unknown"),
        NONE,
        YELLOW,
        message_type,
        NONE,
        RED,
        severity,
        NONE,
        callback_data.message
    );
}
