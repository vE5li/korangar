use chrono;

use std::time::SystemTime;

use super::stack::*;
use super::print::*;
use super::symbols::*;
use super::colors::*;

pub struct Timer {
    start_time: SystemTime,
    name: String,
}

impl Timer {

    pub fn new(name: &'static str) -> Self {
        return Self::new_dynamic(String::from(name));
    }

    pub fn new_dynamic(name: String) -> Self {

        if stack_size() == 0 {
            let timestamp = chrono::offset::Local::now().time().format("%H:%M:%S").to_string();
            print_debug_prefix!("[{}{}{}] {}{}{}", red(), timestamp, none(), yellow(), name, none());
        } else {
            print_debug_prefix!("{}{}{}", yellow(), name, none());
        }

        increment_stack(2);

        return Self {
            start_time: SystemTime::now(),
            name,
        }
    }

    pub fn stop(self) {

        if stack_size() > 0 && get_message_count() == 0 {
            decrement_stack();
            println!(" ({}{}ms{})", cyan(), self.start_time.elapsed().unwrap().as_millis(), none());
        } else {
            decrement_stack();
            print_debug!("{}{}{} {} {}completed{} ({}{}ms{})", yellow(), self.name, none(), arrow_symbol(), green(), none(), cyan(), self.start_time.elapsed().unwrap().as_millis(), none());
        }

        if stack_size() == 0 {
            println!();
        }
    }
}
