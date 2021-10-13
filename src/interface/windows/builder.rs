use cgmath::Vector2;

pub struct WindowBuilder {
    counter: usize,
    window_width: f32,
    border_width: f32,
    left_offset: f32,
    top_offset: f32,
    row_height: f32,
    gap: f32,
}

impl WindowBuilder {

    pub fn new(window_width: f32) -> Self {

        let counter = 0;
        let border_width = 2.5;
        let top_offset = 20.0;
        let left_offset = 0.0;
        let row_height = 0.0;
        let gap = 2.0;

        return Self { counter, window_width, border_width, top_offset, left_offset, row_height, gap };
    }

    pub fn inner_width(&self) -> f32 {
        return self.window_width - self.border_width * 2.0;
    }

    pub fn new_row(&mut self) {

        self.top_offset += self.row_height + self.gap;
        self.left_offset = 0.0;
        self.row_height = 0.0;
    }

    pub fn position(&mut self, size: Vector2<f32>) -> Vector2<f32> {

        if self.left_offset + size.x > self.inner_width() {
            self.new_row();
        }

        let position = Vector2::new(self.border_width + self.left_offset, self.top_offset);

        self.left_offset += size.x + self.gap;
        self.row_height = size.y;

        return position;
    }

    pub fn final_size(&mut self) -> Vector2<f32> {
        self.new_row();

        return Vector2::new(self.window_width, self.top_offset);
    }

    pub fn unique_identifier(&mut self) -> usize {
        let index = self.counter;
        self.counter += 1;
        return index;
    }
}
