use cosmic_text::Attrs;

use crate::graphics::Color;

const RESET_COLOR_CODE: &str = "000000";
const HIGHLIGHT_COLOR_CODE: &str = "000001";

pub(crate) struct ColorSpanIterator<'r, 's> {
    text: &'s str,
    default_color: cosmic_text::Color,
    highlight_color: cosmic_text::Color,
    attributes: Attrs<'r>,
    position: usize,
}

impl<'r, 's> ColorSpanIterator<'r, 's> {
    pub(crate) fn new(text: &'s str, default_color: Color, highlight_color: Color, attributes: Attrs<'r>) -> Self {
        Self {
            text,
            default_color: default_color.into(),
            highlight_color: highlight_color.into(),
            attributes,
            position: 0,
        }
    }
}

impl<'r, 's> Iterator for ColorSpanIterator<'r, 's> {
    type Item = (&'s str, Attrs<'r>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.text.len() {
            return None;
        }

        let start_position = self.position;
        let mut current_position = self.position;
        let text = &self.text[current_position..];

        while let Some(color_position) = text[current_position - self.position..].find('^') {
            let absolute_color_position = current_position + color_position;

            // Change the font color if the color value is valid.
            if absolute_color_position + 7 <= self.text.len() {
                let potential_color = &self.text[absolute_color_position + 1..absolute_color_position + 7];
                if potential_color.chars().all(|c| c.is_ascii_hexdigit()) {
                    if absolute_color_position > start_position {
                        let span_text = &self.text[start_position..absolute_color_position];
                        self.position = absolute_color_position;
                        return Some((span_text, self.attributes.clone()));
                    }

                    self.position = absolute_color_position + 7;
                    self.attributes.color_opt = match potential_color {
                        RESET_COLOR_CODE => Some(self.default_color),
                        HIGHLIGHT_COLOR_CODE => Some(self.highlight_color),
                        code => Some(Color::rgb_hex(code).into()),
                    };

                    return self.next();
                }
            }

            // Invalid color code - continue searching.
            current_position = absolute_color_position + 1;
            if current_position >= self.text.len() {
                break;
            }
        }

        // Return remaining text.
        if self.position < self.text.len() {
            let span_text = &self.text[self.position..];
            self.position = self.text.len();
            Some((span_text, self.attributes.clone()))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_color_change() {
        let attributes = Attrs::new();
        let text = "Hello ^FF0000Red ^00FF00Green";
        let spans: Vec<_> = ColorSpanIterator::new(text, Color::BLACK, Color::WHITE, attributes).collect();

        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].0, "Hello ");
        assert_eq!(spans[1].0, "Red ");
        assert_eq!(spans[2].0, "Green");
    }

    #[test]
    fn test_reset_to_default_color() {
        let attributes = Attrs::new();
        let text = "^FF0000Red ^000000Default";
        let spans: Vec<_> = ColorSpanIterator::new(text, Color::BLACK, Color::WHITE, attributes).collect();

        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].0, "Red ");
        assert_eq!(spans[1].0, "Default");
    }

    #[test]
    fn test_invalid_color_codes() {
        let attributes = Attrs::new();
        let text = "^FFInvalid ^FF00 ^FFFF00Valid";
        let spans: Vec<_> = ColorSpanIterator::new(text, Color::BLACK, Color::WHITE, attributes).collect();

        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].0, "^FFInvalid ^FF00 ");
        assert_eq!(spans[1].0, "Valid");
    }

    #[test]
    fn test_empty_text_between_colors() {
        let attributes = Attrs::new();
        let text = "^FF0000^00FF00^0000FF";
        let spans: Vec<_> = ColorSpanIterator::new(text, Color::BLACK, Color::WHITE, attributes).collect();

        assert_eq!(spans.len(), 0);
    }

    #[test]
    fn test_color_code_at_end() {
        let attributes = Attrs::new();
        let text = "Text^FF0000";
        let spans: Vec<_> = ColorSpanIterator::new(text, Color::BLACK, Color::WHITE, attributes).collect();

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].0, "Text");
    }

    #[test]
    fn test_caret_at_end() {
        let attributes = Attrs::new();
        let text = "Normal text ^FFFF00Colored text^";
        let spans: Vec<_> = ColorSpanIterator::new(text, Color::BLACK, Color::WHITE, attributes).collect();

        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].0, "Normal text ");
        assert_eq!(spans[1].0, "Colored text^");
    }

    #[test]
    fn test_consecutive_color_changes() {
        let attributes = Attrs::new();
        let text = "^AAAAAA^BBBBBBtext^CCCCCCmore^DDDDDDlast";
        let spans: Vec<_> = ColorSpanIterator::new(text, Color::BLACK, Color::WHITE, attributes).collect();

        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].0, "text");
        assert_eq!(spans[1].0, "more");
        assert_eq!(spans[2].0, "last");
    }

    #[test]
    fn test_empty_input() {
        let attributes = Attrs::new();
        let text = "";
        let spans: Vec<_> = ColorSpanIterator::new(text, Color::BLACK, Color::WHITE, attributes).collect();

        assert_eq!(spans.len(), 0);
    }

    #[test]
    fn test_highlight_color() {
        let attributes = Attrs::new();
        let text = "^000001Highlighted ^000000Default";
        let spans: Vec<_> = ColorSpanIterator::new(text, Color::BLACK, Color::WHITE, attributes).collect();

        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].0, "Highlighted ");
        assert_eq!(spans[1].0, "Default");
    }
}
