use cgmath::Vector2;

pub struct InterfaceState {
    offsets: Vec<Vector2<f32>>,
}

impl InterfaceState {

    pub fn new() -> Self {
        let offsets = Vec::new();
        return Self { offsets };
    }

    pub fn register_draggable(&mut self) -> usize {
        let index = self.offsets.len();
        self.offsets.push(Vector2::new(0.0, 0.0));
        return index;
    }

    pub fn move_offset(&mut self, index: usize, offset: Vector2<f32>) {
        self.offsets[index] += offset;
    }

    pub fn get_offset(&self, index: usize) -> Vector2<f32> {
        return self.offsets[index];
    }
}
