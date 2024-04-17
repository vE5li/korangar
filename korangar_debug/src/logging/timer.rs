use std::time::SystemTime;

use super::colors::*;
use super::print::*;
use super::stack::*;
use super::symbols::*;

pub struct Timer {
    start_time: SystemTime,
    completed: bool,
    name: String,
}

impl Timer {
    pub fn new(name: &'static str) -> Self {
        Self::new_dynamic(String::from(name))
    }

    pub fn new_dynamic(name: String) -> Self {
        if stack_size() == 0 {
            let timestamp = chrono::offset::Local::now().time().format("%H:%M:%S").to_string();
            print_debug_prefix!("[{}] {}", timestamp.red(), name);
        } else {
            print_debug_prefix!("{}", name);
        }

        increment_stack(2);

        let start_time = SystemTime::now();
        let completed = false;

        Self {
            start_time,
            completed,
            name,
        }
    }

    pub fn stop(mut self) {
        if stack_size() > 0 && get_message_count() == 0 {
            decrement_stack();
            println!(" ({})", format!("{}ms", self.start_time.elapsed().unwrap().as_millis()).cyan());
        } else {
            decrement_stack();
            print_debug!(
                "{} {} {} ({})",
                self.name,
                ARROW,
                "completed".green(),
                format!("{}ms", self.start_time.elapsed().unwrap().as_millis()).cyan(),
            );
        }

        if stack_size() == 0 {
            println!();
        }

        self.completed = true;
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        if !self.completed {
            if stack_size() > 0 && get_message_count() == 0 {
                decrement_stack();
                println!(" ({})", format!("{}ms", self.start_time.elapsed().unwrap().as_millis()).cyan());
            } else {
                decrement_stack();
                print_debug!(
                    "{} {} {} ({})",
                    self.name,
                    ARROW,
                    "failed".red(),
                    format!("{}ms", self.start_time.elapsed().unwrap().as_millis()).cyan(),
                );
            }

            if stack_size() == 0 {
                println!();
            }
        }
    }
}
