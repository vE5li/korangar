use derive_new::new;

#[derive(Copy, Clone, Debug, new)]
pub struct Key {
    #[new(default)]
    is_down: bool,
    #[new(default)]
    was_down: bool,
    #[new(default)]
    is_pressed: bool,
    #[new(default)]
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
        return self.is_down;
    }

    pub fn pressed(&self) -> bool {
        return self.is_pressed;
    }

    pub fn released(&self) -> bool {
        return self.is_released;
    }
}
