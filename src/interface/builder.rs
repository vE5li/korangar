/// A generic builder parameter that has not been set.
pub struct Unset;
/// A generic builder parameter that has been set.
pub struct Set;
/// A generic builder parameter that has been set to a value T.
pub struct With<T>(T);

impl<T> With<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn take(self) -> T {
        self.0
    }
}
