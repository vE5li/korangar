use std::sync::{LazyLock, Mutex};

struct StackItem {
    pub message_count: usize,
    pub size: usize,
}

impl StackItem {
    pub fn new(message_count: usize, size: usize) -> Self {
        Self { message_count, size }
    }
}

static STACK: LazyLock<Mutex<Vec<StackItem>>> = LazyLock::new(|| Mutex::new(Vec::new()));

pub fn stack_size() -> usize {
    match STACK.try_lock() {
        Ok(ref mut mutex) => mutex.len(),
        Err(..) => panic!(),
    }
}

pub fn message_offset() -> usize {
    match STACK.try_lock() {
        Ok(ref mut mutex) => mutex.iter().map(|item| item.size).sum(),
        Err(..) => panic!(),
    }
}

pub fn increment_stack(size: usize) {
    match STACK.try_lock() {
        Ok(ref mut mutex) => mutex.push(StackItem::new(0, size)),
        Err(..) => panic!(),
    }
}

pub fn decrement_stack() {
    match STACK.try_lock() {
        Ok(ref mut mutex) => mutex.pop(),
        Err(..) => panic!(),
    };
}

pub fn increment_message_count() {
    match STACK.try_lock() {
        Ok(ref mut mutex) => mutex.last_mut().unwrap().message_count += 1,
        Err(..) => panic!(),
    }
}

pub fn get_message_count() -> usize {
    match STACK.try_lock() {
        Ok(ref mut mutex) => mutex.last_mut().unwrap().message_count,
        Err(..) => panic!(),
    }
}
