#[derive(Copy, Clone, Debug, Default)]
pub struct Key {
    is_down: bool,
    was_down: bool,
    is_pressed: bool,
    is_released: bool,
}

impl Key {

    pub fn reset(&mut self) {

        self.update();
        self.is_down = false;
    }

    pub fn set_down(&mut self, is_down: bool) {
        self.is_down = is_down;
    }

    pub fn update(&mut self) {

        self.is_pressed = self.is_down && !self.was_down;
        self.is_released = !self.is_down && self.was_down;
        self.was_down = self.is_down;
    }

    pub fn down(&self) -> bool {
        self.is_down
    }

    pub fn pressed(&self) -> bool {
        self.is_pressed
    }

    pub fn released(&self) -> bool {
        self.is_released
    }
}
