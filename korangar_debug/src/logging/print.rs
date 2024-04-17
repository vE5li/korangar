use crate::logging::stack::{get_message_count, increment_message_count, message_offset, stack_size};
use crate::logging::symbols::{ARROW, NEWLINE};

pub macro print_debug {
    ($format:expr) => (print_indented(String::from($format), true)),
    ($format:expr, $($arguments:tt)*) => (print_indented(format!($format, $($arguments)*), true)),
}

pub(crate) macro print_debug_prefix {
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
