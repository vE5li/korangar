#[derive(Copy, Clone, Debug)]
pub struct Key {
    is_down: bool,
    was_down: bool,
    is_pressed: bool,
    is_released: bool,
}

impl Key {

    pub fn new() -> Self {

        let is_down = false;
        let was_down = false;
        let is_pressed = false;
        let is_released = false;

        return Self { is_down, was_down, is_pressed, is_released };
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
        return self.is_down;
    }

    pub fn pressed(&self) -> bool {
        return self.is_pressed;
    }

    pub fn released(&self) -> bool {
        return self.is_released;
    }
}
