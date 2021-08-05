use chrono;

use std::time::SystemTime;

use super::stack::*;
use super::print::*;
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
            print_debug!("[ {}{}{} ] started ({}{}{})", yellow(), name, none(), red(), timestamp, none());
        } else {
            print_debug!("[ {}{}{} ] started", yellow(), name, none());
        }

        increment_stack(1);

        return Self {
            start_time: SystemTime::now(),
            name: name,
        }
    }

    pub fn stop(self) {
        decrement_stack(1);

        print_debug!("[ {}{}{} ] {}completed {}({}{}ms{})", yellow(), self.name, none(), green(), none(), cyan(), self.start_time.elapsed().unwrap().as_millis(), none());

        if stack_size() == 0 {
            println!();
        }
    }
}
