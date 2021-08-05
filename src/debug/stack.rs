use std::sync::Mutex;

lazy_static! {
    static ref INDENTATION: Mutex<usize> = Mutex::new(0);
}

pub fn stack_size() -> usize {
    match INDENTATION.try_lock() {
        Ok(ref mut mutex) => return **mutex,
        Err(..) => panic!(),
    };
}

pub fn increment_stack(size: usize) {
    match INDENTATION.try_lock() {
        Ok(ref mut mutex) => **mutex += size,
        Err(..) => panic!(),
    };
}

pub fn decrement_stack(size: usize) {
    match INDENTATION.try_lock() {
        Ok(ref mut mutex) => **mutex -= size,
        Err(..) => panic!(),
    };
}
