use std::cell::RefCell;
use std::rc::Rc;

use cgmath::Vector2;

use crate::interface::{PartialScreenSize, ScreenPosition, ScreenSize, SizeConstraint};
use crate::loaders::FontLoader;

const ELEMENT_THRESHHOLD: f32 = 1.0000;
const REMAINDER_THRESHHOLD: f32 = 0.0001;

pub struct PlacementResolver {
    font_loader: Rc<RefCell<FontLoader>>,
    available_space: PartialScreenSize,
    base_position: ScreenPosition,
    horizontal_accumulator: f32,
    vertical_offset: f32,
    total_height: f32,
    border: ScreenSize,
    gaps: ScreenSize,
    scaling: f32,
}

impl PlacementResolver {
    pub fn new(
        font_loader: Rc<RefCell<FontLoader>>,
        mut available_space: PartialScreenSize,
        border: ScreenSize,
        gaps: ScreenSize,
        scaling: f32,
    ) -> Self {
        available_space.width -= border.width * scaling * 2.0;
        available_space.height = available_space.height.map(|height| height - border.height * scaling * 2.0);

        let base_position = ScreenPosition::from_size(border * scaling);
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

    pub fn derive(&self, mut available_space: PartialScreenSize, offset: ScreenPosition, border: ScreenSize) -> Self {
        available_space.width -= offset.left + border.width * self.scaling * 2.0;
        available_space.height = available_space.height.map(|height| height - border.height * self.scaling * 2.0);

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
        text_offset: ScreenPosition,
        scaling: f32,
        available_width: f32,
    ) -> Vector2<f32> {
        self.font_loader
            .borrow()
            .get_text_dimensions(text, font_size * scaling, available_width - text_offset.left * scaling)
    }

    pub fn set_gaps(&mut self, gaps: ScreenSize) {
        self.gaps = gaps;
    }

    pub fn get_available(&self) -> PartialScreenSize {
        self.available_space
    }

    pub fn get_remaining(&self) -> PartialScreenSize {
        let remaining_width = self.available_space.width - self.horizontal_accumulator;
        let remaining_height = self
            .available_space
            .height
            .map(|height| height - self.total_height - self.vertical_offset);

        PartialScreenSize::new(remaining_width, remaining_height)
    }

    pub fn newline(&mut self) {
        self.total_height += self.vertical_offset + self.gaps.height * self.scaling;
        self.base_position.top += self.vertical_offset + self.gaps.height * self.scaling;
        self.horizontal_accumulator = 0.0;
        self.vertical_offset = 0.0;
    }

    pub fn register_height(&mut self, height: f32) {
        self.vertical_offset = f32::max(self.vertical_offset, height);
    }

    pub fn allocate(&mut self, size_constraint: &SizeConstraint) -> (PartialScreenSize, ScreenPosition) {
        let is_width_absolute = size_constraint.width.is_absolute();
        let gaps_add = match is_width_absolute {
            true => self.gaps.width * 2.0,
            false => 0.0,
        };

        let mut remaining = self.get_remaining();
        let mut size = size_constraint.resolve_partial(self.available_space, remaining, self.scaling);
        let mut gaps_subtract = 0.0;

        if remaining.width < size.width - REMAINDER_THRESHHOLD {
            self.newline();
            remaining = self.get_remaining();

            if size_constraint.width.is_remaining() || size_constraint.height.is_remaining() {
                size = size_constraint.resolve_partial(self.available_space, remaining, self.scaling);
            }

            size.width = f32::min(size.width, self.available_space.width);
        }

        if self.horizontal_accumulator > ELEMENT_THRESHHOLD {
            match is_width_absolute {
                true => {}
                false => gaps_subtract += self.gaps.width * self.scaling,
            }
        }

        let position = ScreenPosition {
            left: self.base_position.left + self.horizontal_accumulator + gaps_subtract,
            top: self.base_position.top,
        };

        self.horizontal_accumulator += size.width + gaps_add;

        if let Some(height) = size.height {
            self.register_height(height);
        }

        if remaining.width - size.width > ELEMENT_THRESHHOLD {
            match is_width_absolute {
                true => {}
                false => gaps_subtract += self.gaps.width * self.scaling,
            }
        }

        size.width -= gaps_subtract;
        (size, position)
    }

    pub fn allocate_right(&mut self, size_constraint: &SizeConstraint) -> (PartialScreenSize, ScreenPosition) {
        let mut remaining = self.get_remaining();
        let mut size = size_constraint.resolve_partial(self.available_space, remaining, self.scaling);

        if remaining.width < size.width - REMAINDER_THRESHHOLD + self.gaps.width * self.scaling {
            self.newline();
            remaining = self.get_remaining();

            if size_constraint.width.is_remaining() || size_constraint.height.is_remaining() {
                size = size_constraint.resolve_partial(self.available_space, remaining, self.scaling);
            }
        }

        let position = ScreenPosition {
            left: self.base_position.left + (self.available_space.width - size.width - self.gaps.width * self.scaling),
            top: self.base_position.top,
        };

        self.horizontal_accumulator += remaining.width;

        if let Some(height) = size.height {
            self.register_height(height);
        }

        (size, position)
    }

    pub fn final_height(self) -> f32 {
        self.total_height + self.vertical_offset + self.border.height * self.scaling
    }
}
