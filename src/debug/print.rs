use super::stack_size;

#[macro_export]
macro_rules! print_debug {
    ($format:expr) => (print_indented(String::from($format)));
    ($format:expr, $($arguments:tt)*) => (print_indented(format!($format, $($arguments)*)));
}

pub fn print_indented(message: String) {
    let indentation = stack_size();

    for _ in 0..indentation {
        print!("  ");
    }

    if indentation != 0 {
        print!("-> ");
    }

    println!("{}", message);
}
