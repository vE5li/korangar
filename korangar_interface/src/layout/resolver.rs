use super::SizeBound;
use super::bound::ParentLimits;
use crate::application::{
    Application, FontLoaderTrait, FontSizeTraitExt, PartialSizeTrait, PositionTrait, PositionTraitExt, ScalingTrait, SizeTrait,
    SizeTraitExt,
};

const ELEMENT_THRESHHOLD: f32 = 1.0000;
const REMAINDER_THRESHHOLD: f32 = 0.0001;

pub struct PlacementResolver<App>
where
    App: Application,
{
    font_loader: App::FontLoader,
    available_space: App::PartialSize,
    parent_limits: ParentLimits,
    base_position: App::Position,
    horizontal_accumulator: f32,
    vertical_offset: f32,
    total_height: f32,
    border: App::Size,
    gaps: App::Size,
    scaling: App::Scaling,
}

impl<App> PlacementResolver<App>
where
    App: Application,
{
    pub fn new(
        font_loader: App::FontLoader,
        screen_size: App::Size,
        window_size: App::Size,
        size_bound: &SizeBound,
        border: App::Size,
        gaps: App::Size,
        scaling: App::Scaling,
    ) -> Self {
        let window_size = window_size.shrink(border.scaled(scaling).doubled());

        // I'm not enirely sure why we need to subtract one border here, but if we
        // don't the `super` bound is too big.
        let parent_limits = ParentLimits::from_bound(size_bound, screen_size.shrink(border.scaled(scaling)), scaling);
        let base_position = App::Position::from_size(border.scaled(scaling));
        let horizontal_accumulator = 0.0;
        let vertical_offset = 0.0;
        let total_height = 0.0;

        let height = (!size_bound.height.is_flexible()).then_some(window_size.height());
        let available_space = App::PartialSize::new(window_size.width(), height);

        Self {
            font_loader,
            available_space,
            parent_limits,
            base_position,
            horizontal_accumulator,
            total_height,
            vertical_offset,
            border,
            gaps,
            scaling,
        }
    }

    pub fn derive(&mut self, size_bound: &SizeBound, offset: App::Position, border: App::Size) -> (Self, App::PartialSize, App::Position) {
        let (size, position) = self.allocate(size_bound);

        let border_with_offset = offset.remaining(border.scaled(self.scaling).doubled());
        let available_space = App::PartialSize::new(
            size.width() - border_with_offset.width(),
            size.height().map(|height| height - border_with_offset.height()),
        );

        let font_loader = self.font_loader.clone();
        let unusable_space = position.remaining(border_with_offset);
        let parent_limits = self.parent_limits.derive(size_bound, available_space, unusable_space, self.scaling);
        let base_position = offset.offset(border.scaled(self.scaling));
        let horizontal_accumulator = 0.0;
        let vertical_offset = 0.0;
        let total_height = 0.0;
        let gaps = self.gaps;
        let scaling = self.scaling;

        let derived_resolver = Self {
            font_loader,
            available_space,
            parent_limits,
            base_position,
            horizontal_accumulator,
            total_height,
            vertical_offset,
            border,
            gaps,
            scaling,
        };

        (derived_resolver, size, position)
    }

    pub fn get_text_dimensions(
        &self,
        text: &str,
        font_size: App::FontSize,
        text_offset: App::Position,
        scaling: App::Scaling,
        available_width: f32,
    ) -> App::Size {
        self.font_loader.get_text_dimensions(
            text,
            font_size.scaled(scaling),
            available_width - text_offset.left() * scaling.get_factor(),
        )
    }

    pub fn set_gaps(&mut self, gaps: App::Size) {
        self.gaps = gaps;
    }

    pub fn get_available(&self) -> App::PartialSize {
        self.available_space
    }

    pub fn get_remaining(&self) -> App::PartialSize {
        let remaining_width = self.available_space.width() - self.horizontal_accumulator;
        let remaining_height = self
            .available_space
            .height()
            .map(|height| height - self.total_height - self.vertical_offset);

        App::PartialSize::new(remaining_width, remaining_height)
    }

    pub fn newline(&mut self) {
        self.total_height += self.vertical_offset + self.gaps.height() * self.scaling.get_factor();
        self.base_position = App::Position::new(
            self.base_position.left(),
            self.base_position.top() + self.vertical_offset + self.gaps.height() * self.scaling.get_factor(),
        );
        self.horizontal_accumulator = 0.0;
        self.vertical_offset = 0.0;
    }

    pub fn register_height(&mut self, height: f32) {
        self.vertical_offset = f32::max(self.vertical_offset, height);
    }

    pub fn allocate(&mut self, size_bound: &SizeBound) -> (App::PartialSize, App::Position) {
        let is_width_absolute = size_bound.width.is_absolute();
        let gaps_add = match is_width_absolute {
            true => self.gaps.width() * 2.0,
            false => 0.0,
        };

        let mut remaining = self.get_remaining();
        let mut size = size_bound.resolve_element::<App::PartialSize>(self.available_space, remaining, &self.parent_limits, self.scaling);
        let mut gaps_subtract = 0.0;

        if remaining.width() < size.width() - REMAINDER_THRESHHOLD {
            self.newline();
            remaining = self.get_remaining();

            if size_bound.width.is_remaining() || size_bound.height.is_remaining() {
                size = size_bound.resolve_element(self.available_space, remaining, &self.parent_limits, self.scaling);
            }

            size = App::PartialSize::new(f32::min(size.width(), self.available_space.width()), size.height());
        }

        if self.horizontal_accumulator > ELEMENT_THRESHHOLD {
            match is_width_absolute {
                true => {}
                false => gaps_subtract += self.gaps.width() * self.scaling.get_factor(),
            }
        }

        let position = App::Position::new(
            self.base_position.left() + self.horizontal_accumulator + gaps_subtract,
            self.base_position.top(),
        );

        self.horizontal_accumulator += size.width() + gaps_add;

        if let Some(height) = size.height() {
            self.register_height(height);
        }

        if remaining.width() - size.width() > ELEMENT_THRESHHOLD {
            match is_width_absolute {
                true => {}
                false => gaps_subtract += self.gaps.width() * self.scaling.get_factor(),
            }
        }

        size = App::PartialSize::new(size.width() - gaps_subtract, size.height());
        (size, position)
    }

    pub fn allocate_right(&mut self, size_bound: &SizeBound) -> (App::PartialSize, App::Position) {
        let mut remaining = self.get_remaining();
        let mut size = size_bound.resolve_element::<App::PartialSize>(self.available_space, remaining, &self.parent_limits, self.scaling);

        if remaining.width() < size.width() - REMAINDER_THRESHHOLD + self.gaps.width() * self.scaling.get_factor() {
            self.newline();
            remaining = self.get_remaining();

            if size_bound.width.is_remaining() || size_bound.height.is_remaining() {
                size = size_bound.resolve_element(self.available_space, remaining, &self.parent_limits, self.scaling);
            }
        }

        let position = App::Position::new(
            self.base_position.left() + (self.available_space.width() - size.width() - self.gaps.width() * self.scaling.get_factor()),
            self.base_position.top(),
        );

        self.horizontal_accumulator += remaining.width();

        if let Some(height) = size.height() {
            self.register_height(height);
        }

        (size, position)
    }

    pub(crate) fn get_parent_limits(&self) -> ParentLimits {
        self.parent_limits
    }

    pub fn final_height(self) -> f32 {
        self.total_height + self.vertical_offset + self.border.height() * 2.0 * self.scaling.get_factor()
    }
}
