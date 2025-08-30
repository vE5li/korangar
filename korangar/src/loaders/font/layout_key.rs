use std::hash::{Hash, Hasher};

use crate::loaders::FontSize;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct LayoutKey {
    pub(super) text: String,
    pub(super) default_color: cosmic_text::Color,
    pub(super) highlight_color: cosmic_text::Color,
    pub(super) font_size: FontSize,
    pub(super) line_height_scale: f32,
    pub(super) layout_width: f32,
}

impl Hash for LayoutKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.text.hash(state);
        self.default_color.hash(state);
        self.highlight_color.hash(state);
        self.font_size.0.to_bits().hash(state);
        self.line_height_scale.to_bits().hash(state);
        self.layout_width.to_bits().hash(state);
    }
}

impl Eq for LayoutKey {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct LayoutKeyRef<'a> {
    pub(super) text: &'a str,
    pub(super) default_color: cosmic_text::Color,
    pub(super) highlight_color: cosmic_text::Color,
    pub(super) font_size: FontSize,
    pub(super) line_height_scale: f32,
    pub(super) layout_width: f32,
}

impl Hash for LayoutKeyRef<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.text.hash(state);
        self.default_color.hash(state);
        self.highlight_color.hash(state);
        self.font_size.0.to_bits().hash(state);
        self.line_height_scale.to_bits().hash(state);
        self.layout_width.to_bits().hash(state);
    }
}

impl Eq for LayoutKeyRef<'_> {}

impl PartialEq<LayoutKey> for LayoutKeyRef<'_> {
    fn eq(&self, other: &LayoutKey) -> bool {
        self.text == other.text
            && self.default_color == other.default_color
            && self.highlight_color == other.highlight_color
            && self.font_size == other.font_size
            && self.line_height_scale == other.line_height_scale
            && self.layout_width == other.layout_width
    }
}

impl PartialEq<LayoutKeyRef<'_>> for LayoutKey {
    fn eq(&self, other: &LayoutKeyRef<'_>) -> bool {
        other.eq(self)
    }
}

impl<'a> LayoutKeyRef<'a> {
    pub(super) fn to_owned(self) -> LayoutKey {
        LayoutKey {
            text: self.text.to_string(),
            default_color: self.default_color,
            highlight_color: self.highlight_color,
            font_size: self.font_size,
            line_height_scale: self.line_height_scale,
            layout_width: self.layout_width,
        }
    }
}
