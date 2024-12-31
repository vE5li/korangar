use std::cell::RefCell;

struct StackItem {
    pub message_count: usize,
    pub size: usize,
}

impl StackItem {
    pub fn new(message_count: usize, size: usize) -> Self {
        Self { message_count, size }
    }
}

#[thread_local]
static STACK: RefCell<Vec<StackItem>> = RefCell::new(Vec::new());

pub fn stack_size() -> usize {
    STACK.borrow().len()
}

pub fn message_offset() -> usize {
    STACK.borrow().iter().map(|item| item.size).sum()
}

pub fn increment_stack(size: usize) {
    STACK.borrow_mut().push(StackItem::new(0, size))
}

pub fn decrement_stack() {
    STACK.borrow_mut().pop();
}

pub fn increment_message_count() {
    STACK.borrow_mut().last_mut().unwrap().message_count += 1;
}

pub fn get_message_count() -> usize {
    STACK.borrow_mut().last_mut().unwrap().message_count
}
