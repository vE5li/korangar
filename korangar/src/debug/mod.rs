#[macro_use]
mod logging;
#[macro_use]
mod profiling;

pub use self::logging::*;
pub use self::profiling::*;

#[cfg(test)]
mod debug_condition {

    use korangar_procedural::debug_condition;

    #[test]
    #[should_panic]
    fn condition_true() {
        #[debug_condition(true)]
        panic!("panic should be called");
    }

    #[test]
    fn condition_false() {
        #[debug_condition(false)]
        panic!("panic should not be called");
    }
}
