use std::cell::RefCell;
use std::rc::Rc;

use cgmath::Vector2;

use crate::interface::{PartialSize, Position, Size, SizeConstraint};
use crate::loaders::FontLoader;

const ELEMENT_THRESHHOLD: f32 = 1.0000;
const REMAINDER_THRESHHOLD: f32 = 0.0001;

pub struct PlacementResolver {
    font_loader: Rc<RefCell<FontLoader>>,
    available_space: PartialSize,
    base_position: Position,
    horizontal_accumulator: f32,
    vertical_offset: f32,
    total_height: f32,
    border: Size,
    gaps: Size,
    scaling: f32,
}

impl PlacementResolver {
    pub fn new(
        font_loader: Rc<RefCell<FontLoader>>,
        mut available_space: PartialSize,
        base_position: Position,
        border: Size,
        gaps: Size,
        scaling: f32,
    ) -> Self {
        available_space.x -= border.x * scaling * 2.0;
        available_space.y = available_space.y.map(|height| height - border.y * scaling * 2.0);

        let base_position = base_position * scaling + border * scaling;
        let horizontal_accumulator = 0.0;
        let vertical_offset = 0.0;
        let total_height = 0.0;

        Self {
            font_loader,
            available_space,
            base_position,
            horizontal_accumulator,
            total_height,
            vertical_offset,
            border,
            gaps,
            scaling,
        }
    }

    pub fn derive(&self, mut available_space: PartialSize, offset: Position, border: Size) -> Self {
        available_space.x -= offset.x + border.x * self.scaling * 2.0;
        available_space.y = available_space.y.map(|height| height - border.y * self.scaling * 2.0);

        let font_loader = self.font_loader.clone();
        let base_position = offset + border * self.scaling;
        let horizontal_accumulator = 0.0;
        let vertical_offset = 0.0;
        let total_height = 0.0;
        let gaps = self.gaps;
        let scaling = self.scaling;

        Self {
            font_loader,
            available_space,
            base_position,
            horizontal_accumulator,
            total_height,
            vertical_offset,
            border,
            gaps,
            scaling,
        }
    }

    pub fn get_text_dimensions(
        &self,
        text: &str,
        font_size: f32,
        text_offset: Vector2<f32>,
        scaling: f32,
        available_width: f32,
    ) -> Vector2<f32> {
        self.font_loader
            .borrow()
            .get_text_dimensions(text, font_size * scaling, available_width - text_offset.x * scaling)
    }

    pub fn set_gaps(&mut self, gaps: Size) {
        self.gaps = gaps;
    }

    pub fn get_available(&self) -> PartialSize {
        self.available_space
    }

    pub fn get_remaining(&self) -> PartialSize {
        let remaining_width = self.available_space.x - self.horizontal_accumulator;
        let remaining_height = self
            .available_space
            .y
            .map(|height| height - self.total_height - self.vertical_offset);

        PartialSize::new(remaining_width, remaining_height)
    }

    pub fn newline(&mut self) {
        self.total_height += self.vertical_offset + self.gaps.y * self.scaling;
        self.base_position.y += self.vertical_offset + self.gaps.y * self.scaling;
        self.horizontal_accumulator = 0.0;
        self.vertical_offset = 0.0;
    }

    pub fn register_height(&mut self, height: f32) {
        self.vertical_offset = f32::max(self.vertical_offset, height);
    }

    pub fn allocate(&mut self, size_constraint: &SizeConstraint) -> (PartialSize, Position) {
        let is_width_absolute = size_constraint.width.is_absolute();
        let gaps_add = match is_width_absolute {
            true => self.gaps.x * 2.0,
            false => 0.0,
        };

        let mut remaining = self.get_remaining();
        let mut size = size_constraint.resolve_partial(self.available_space, remaining, self.scaling);
        let mut gaps_subtract = 0.0;

        if remaining.x < size.x - REMAINDER_THRESHHOLD {
            self.newline();
            remaining = self.get_remaining();

            if size_constraint.width.is_remaining() || size_constraint.height.is_remaining() {
                size = size_constraint.resolve_partial(self.available_space, remaining, self.scaling);
            }

            size.x = f32::min(size.x, self.available_space.x);
        }

        if self.horizontal_accumulator > ELEMENT_THRESHHOLD {
            match is_width_absolute {
                true => {}
                false => gaps_subtract += self.gaps.x * self.scaling,
            }
        }

        let position = Vector2::new(
            self.base_position.x + self.horizontal_accumulator + gaps_subtract,
            self.base_position.y,
        );

        self.horizontal_accumulator += size.x + gaps_add;

        if let Some(height) = size.y {
            self.register_height(height);
        }

        if remaining.x - size.x > ELEMENT_THRESHHOLD {
            match is_width_absolute {
                true => {}
                false => gaps_subtract += self.gaps.x * self.scaling,
            }
        }

        size.x -= gaps_subtract;
        (size, position)
    }

    pub fn allocate_right(&mut self, size_constraint: &SizeConstraint) -> (PartialSize, Position) {
        let mut remaining = self.get_remaining();
        let mut size = size_constraint.resolve_partial(self.available_space, remaining, self.scaling);

        if remaining.x < size.x - REMAINDER_THRESHHOLD + self.gaps.x * self.scaling {
            self.newline();
            remaining = self.get_remaining();

            if size_constraint.width.is_remaining() || size_constraint.height.is_remaining() {
                size = size_constraint.resolve_partial(self.available_space, remaining, self.scaling);
            }
        }

        let position = Vector2::new(
            self.base_position.x + (self.available_space.x - size.x - self.gaps.x * self.scaling),
            self.base_position.y,
        );

        self.horizontal_accumulator += remaining.x;

        if let Some(height) = size.y {
            self.register_height(height);
        }

        (size, position)
    }

    pub fn final_height(self) -> f32 {
        self.total_height + self.vertical_offset + self.border.y * self.scaling
    }
}
