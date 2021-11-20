use cgmath::Vector2;

use graphics::Color;

use super::super::*;

const FRAME_WIDTH: f32 = 1.0;
const TOP_FRAME_HEIGHT: f32 = 22.0;
const FRAME_COLOR: Color = Color::new(30, 30, 30);
const TITLE_TEXT_COLOR: Color = Color::new(70, 70, 70);
const TITLE_TEXT_OFFSET: Vector2<f32> = Vector2::new(10.0, 4.0);
const TITLE_TEXT_SIZE: f32 = 13.0;

const ELEMENT_GAP: f32 = 4.0;

pub struct WindowBuilder {
    counter: usize,
    window_width: f32,
    left_offset: f32,
    top_offset: f32,
    row_height: f32,
}

impl WindowBuilder {

    pub fn new(window_width: f32) -> Self {

        let counter = 0;
        let top_offset = 0.0;
        let left_offset = 0.0;
        let row_height = 0.0;

        return Self { counter, window_width, top_offset, left_offset, row_height };
    }

    pub fn reset(&mut self) {
        self.top_offset = 0.0;
        self.left_offset = 0.0;
        self.row_height = 0.0;
    }

    pub fn inner_width(&self) -> f32 {
        return self.window_width - FRAME_WIDTH * 2.0;
    }

    pub fn remaining_width(&self) -> f32 {
        return self.inner_width() - self.left_offset;
    }

    pub fn new_row(&mut self) {
        self.top_offset += self.row_height + ELEMENT_GAP;
        self.left_offset = 0.0;
        self.row_height = 0.0;
    }

    pub fn new_row_spaced(&mut self, spacing: f32) {
        self.top_offset += self.row_height + ELEMENT_GAP + spacing;
        self.left_offset = 0.0;
        self.row_height = 0.0;
    }

    pub fn position(&mut self, size: Vector2<f32>) -> Vector2<f32> {

        if self.left_offset + size.x > self.inner_width() {
            self.new_row();
        }

        let position = Vector2::new(self.left_offset, self.top_offset);

        self.left_offset += size.x + ELEMENT_GAP;

        if size.y > self.row_height {
            self.row_height = size.y;
        }

        return position;
    }

    pub fn unique_identifier(&mut self) -> usize {
        let index = self.counter;
        self.counter += 1;
        return index;
    }

    fn finalize(&mut self) {
        if self.row_height != 0.0 {
            self.new_row();
        }
    }

    fn window_background(&mut self, elements: Vec<Element>) -> Component {

        let element_index = self.unique_identifier();
        let position = Vector2::new(FRAME_WIDTH, TOP_FRAME_HEIGHT);
        let size = Vector2::new(self.inner_width(), self.top_offset - FRAME_WIDTH);

        //let background = Component::Rectangle(RectangleComponent::new(Vector2::new(0.0, 0.0), size, BACKGROUND_COLOR, BACKGROUND_COLOR));
        let container = Component::Container(ContainerComponent::new(elements));
        let hoverable = Component::Hoverable(HoverableComponent::new(size));

        let components = vec![/*background,*/ hoverable, container];
        let elements = vec![Element::new(components, element_index, position)];
        return Component::Container(ContainerComponent::new(elements));
    }

    pub fn framed_window(&mut self, interface_state: &mut InterfaceState, title: &str, elements: Vec<Element>, position: Vector2<f32>) -> Element {

        self.finalize();

        let element_index = self.unique_identifier();
        let frame_position = Vector2::new(0.0, 0.0);
        let frame_size = Vector2::new(self.window_width, self.top_offset + TOP_FRAME_HEIGHT);
        let frame_corner_radius = Vector4::new(8.0, 8.0, 8.0, 8.0);

        let frame = Component::Rectangle(RectangleComponent::new(frame_position, frame_size, frame_corner_radius, FRAME_COLOR, FRAME_COLOR));
        let title_text = Component::Text(TextComponent::new(TITLE_TEXT_OFFSET, title.to_string(), TITLE_TEXT_COLOR, TITLE_TEXT_SIZE));
        let hoverable = Component::Hoverable(HoverableComponent::new(frame_size));
        let draggable = Component::Draggable(DraggableComponent::new(interface_state));
        let background = self.window_background(elements);

        let components = vec![frame, title_text, hoverable, draggable, background];
        return Element::new(components, element_index, position);
    }
}
