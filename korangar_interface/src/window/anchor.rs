#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::application::{Application, ClipTrait, CornerRadiusTrait, PositionTrait, RenderLayer, SizeTrait};

macro_rules! anchor_color {
    ($anchor_point:expr, $anchor_color:expr, $closest_anchor_color:expr, $name:ident) => {
        match $anchor_point {
            AnchorPoint::$name => $closest_anchor_color,
            _ => $anchor_color,
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AnchorPoint {
    Center,
    TopLeft,
    TopCenter,
    TopRight,
    CenterRight,
    BottomRight,
    BottomCenter,
    BottomLeft,
    CenterLeft,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Anchor<App>
where
    App: Application,
{
    anchor_point: AnchorPoint,
    offset: App::Position,
}

impl<App> Default for Anchor<App>
where
    App: Application,
{
    fn default() -> Self {
        // By default, windows start out in the middle of the screen.
        Self {
            anchor_point: AnchorPoint::Center,
            offset: App::Position::new(0.0, 0.0),
        }
    }
}

impl<App> Clone for Anchor<App>
where
    App: Application,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<App> Copy for Anchor<App> where App: Application {}

impl<App> Anchor<App>
where
    App: Application,
{
    pub fn to_position(&self, window_space: App::Size) -> App::Position {
        let half_width = window_space.width() / 2.0;
        let half_height = window_space.height() / 2.0;

        match self.anchor_point {
            AnchorPoint::Center => App::Position::new(half_width + self.offset.left(), half_height + self.offset.top()),
            AnchorPoint::TopLeft => App::Position::new(self.offset.left(), self.offset.top()),
            AnchorPoint::TopCenter => App::Position::new(half_width + self.offset.left(), self.offset.top()),
            AnchorPoint::TopRight => App::Position::new(window_space.width() + self.offset.left(), self.offset.top()),
            AnchorPoint::CenterRight => App::Position::new(window_space.width() + self.offset.left(), half_height + self.offset.top()),
            AnchorPoint::BottomRight => App::Position::new(
                window_space.width() + self.offset.left(),
                window_space.height() + self.offset.top(),
            ),
            AnchorPoint::BottomCenter => App::Position::new(half_width + self.offset.left(), window_space.height() + self.offset.top()),
            AnchorPoint::BottomLeft => App::Position::new(self.offset.left(), window_space.height() + self.offset.top()),
            AnchorPoint::CenterLeft => App::Position::new(self.offset.left(), half_height + self.offset.top()),
        }
    }

    pub fn update(&mut self, window_space: App::Size, position: App::Position, window_size: App::Size, display_height: f32) {
        let center = (
            Anchor {
                offset: App::Position::new(
                    position.left() - window_space.width() / 2.0,
                    position.top() - window_space.height() / 2.0,
                ),
                anchor_point: AnchorPoint::Center,
            },
            App::Position::new(
                position.left() - (window_space.width() - window_size.width()) / 2.0,
                position.top() - (window_space.height() - display_height) / 2.0,
            ),
        );

        let top_left = (
            Anchor {
                offset: position,
                anchor_point: AnchorPoint::TopLeft,
            },
            position,
        );

        let top_center = (
            Anchor {
                offset: App::Position::new(position.left() - window_space.width() / 2.0, position.top()),
                anchor_point: AnchorPoint::TopCenter,
            },
            App::Position::new(
                position.left() - (window_space.width() - window_size.width()) / 2.0,
                position.top(),
            ),
        );

        let top_right = (
            Anchor {
                offset: App::Position::new(position.left() - window_space.width(), position.top()),
                anchor_point: AnchorPoint::TopRight,
            },
            App::Position::new(position.left() - window_space.width() + window_size.width(), position.top()),
        );

        let center_right = (
            Anchor {
                offset: App::Position::new(
                    position.left() - window_space.width(),
                    position.top() - window_space.height() / 2.0,
                ),
                anchor_point: AnchorPoint::CenterRight,
            },
            App::Position::new(
                position.left() - window_space.width() + window_size.width(),
                position.top() - (window_space.height() - display_height) / 2.0,
            ),
        );

        let bottom_right = (
            Anchor {
                offset: App::Position::new(position.left() - window_space.width(), position.top() - window_space.height()),
                anchor_point: AnchorPoint::BottomRight,
            },
            App::Position::new(
                position.left() - window_space.width() + window_size.width(),
                position.top() - window_space.height() + display_height,
            ),
        );

        let bottom_center = (
            Anchor {
                offset: App::Position::new(
                    position.left() - window_space.width() / 2.0,
                    position.top() - window_space.height(),
                ),
                anchor_point: AnchorPoint::BottomCenter,
            },
            App::Position::new(
                position.left() - (window_space.width() - window_size.width()) / 2.0,
                position.top() - window_space.height() + display_height,
            ),
        );

        let bottom_left = (
            Anchor {
                offset: App::Position::new(position.left(), position.top() - window_space.height()),
                anchor_point: AnchorPoint::BottomLeft,
            },
            App::Position::new(position.left(), position.top() - window_space.height() + display_height),
        );

        let center_left = (
            Anchor {
                offset: App::Position::new(position.left(), position.top() - window_space.height() / 2.0),
                anchor_point: AnchorPoint::CenterLeft,
            },
            App::Position::new(position.left(), position.top() - (window_space.height() - display_height) / 2.0),
        );

        *self = [
            center,
            top_left,
            top_center,
            top_right,
            center_right,
            bottom_right,
            bottom_center,
            bottom_left,
            center_left,
        ]
        .into_iter()
        .min_by_key(|(_, distance)| distance.left().abs() as usize + distance.top().abs() as usize)
        .map(|(anchor, _)| anchor)
        .unwrap();
    }

    pub fn render_window_anchors(
        &self,
        renderer: &App::Renderer,
        anchor_color: App::Color,
        closest_anchor_color: App::Color,
        window_position: App::Position,
        window_size: App::Size,
    ) {
        let dot_width = 10.0;
        let wide_dot_width = 40.0;
        let dot_size = App::Size::new(dot_width, dot_width);
        let screen_clip = App::Clip::unbound();

        renderer.render_rectangle(
            App::Position::new(
                window_position.left() + (window_size.width() - dot_width) / 2.0,
                window_position.top() + (window_size.height() - dot_width) / 2.0,
            ),
            dot_size,
            screen_clip,
            App::CornerRadius::new(dot_width, dot_width, dot_width, dot_width),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, Center),
        );
        renderer.render_rectangle(
            window_position,
            dot_size,
            screen_clip,
            App::CornerRadius::new(0.0, 0.0, dot_width, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, TopLeft),
        );
        renderer.render_rectangle(
            App::Position::new(
                window_position.left() + (window_size.width() - wide_dot_width) / 2.0,
                window_position.top() + 0.0,
            ),
            App::Size::new(wide_dot_width, dot_width),
            screen_clip,
            App::CornerRadius::new(0.0, 0.0, dot_width, dot_width),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, TopCenter),
        );
        renderer.render_rectangle(
            App::Position::new(window_position.left() + window_size.width() - dot_width, window_position.top()),
            dot_size,
            screen_clip,
            App::CornerRadius::new(0.0, 0.0, 0.0, dot_width),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, TopRight),
        );
        renderer.render_rectangle(
            App::Position::new(
                window_position.left() + window_size.width() - dot_width,
                window_position.top() + (window_size.height() - wide_dot_width) / 2.0,
            ),
            App::Size::new(dot_width, wide_dot_width),
            screen_clip,
            App::CornerRadius::new(dot_width, 0.0, 0.0, dot_width),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, CenterRight),
        );
        renderer.render_rectangle(
            App::Position::new(
                window_position.left() + window_size.width() - dot_width,
                window_position.top() + window_size.height() - dot_width,
            ),
            dot_size,
            screen_clip,
            App::CornerRadius::new(dot_width, 0.0, 0.0, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, BottomRight),
        );
        renderer.render_rectangle(
            App::Position::new(
                window_position.left() + (window_size.width() - wide_dot_width) / 2.0,
                window_position.top() + window_size.height() - dot_width,
            ),
            App::Size::new(wide_dot_width, dot_width),
            screen_clip,
            App::CornerRadius::new(dot_width, dot_width, 0.0, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, BottomCenter),
        );
        renderer.render_rectangle(
            App::Position::new(window_position.left(), window_position.top() + window_size.height() - dot_width),
            dot_size,
            screen_clip,
            App::CornerRadius::new(0.0, dot_width, 0.0, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, BottomLeft),
        );
        renderer.render_rectangle(
            App::Position::new(
                window_position.left(),
                window_position.top() + (window_size.height() - wide_dot_width) / 2.0,
            ),
            App::Size::new(dot_width, wide_dot_width),
            screen_clip,
            App::CornerRadius::new(0.0, dot_width, dot_width, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, CenterLeft),
        );
    }

    pub fn render_screen_anchors(
        &self,
        renderer: &App::Renderer,
        anchor_color: App::Color,
        closest_anchor_color: App::Color,
        available_space: App::Size,
    ) {
        let dot_width = 10.0;
        let wide_dot_width = 60.0;
        let dot_size = App::Size::new(dot_width, dot_width);
        let screen_clip = App::Clip::unbound();

        renderer.render_rectangle(
            App::Position::new(
                (available_space.width() - dot_width) / 2.0,
                (available_space.height() - dot_width) / 2.0,
            ),
            dot_size,
            screen_clip,
            App::CornerRadius::new(dot_width, dot_width, dot_width, dot_width),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, Center),
        );
        renderer.render_rectangle(
            App::Position::new(0.0, 0.0),
            dot_size,
            screen_clip,
            App::CornerRadius::new(0.0, 0.0, dot_width, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, TopLeft),
        );
        renderer.render_rectangle(
            App::Position::new((available_space.width() - wide_dot_width) / 2.0, 0.0),
            App::Size::new(wide_dot_width, dot_width),
            screen_clip,
            App::CornerRadius::new(0.0, 0.0, dot_width, dot_width),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, TopCenter),
        );
        renderer.render_rectangle(
            App::Position::new(available_space.width() - dot_width, 0.0),
            dot_size,
            screen_clip,
            App::CornerRadius::new(0.0, 0.0, 0.0, dot_width),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, TopRight),
        );
        renderer.render_rectangle(
            App::Position::new(
                available_space.width() - dot_width,
                (available_space.height() - wide_dot_width) / 2.0,
            ),
            App::Size::new(dot_width, wide_dot_width),
            screen_clip,
            App::CornerRadius::new(dot_width, 0.0, 0.0, dot_width),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, CenterRight),
        );
        renderer.render_rectangle(
            App::Position::new(available_space.width() - dot_width, available_space.height() - dot_width),
            dot_size,
            screen_clip,
            App::CornerRadius::new(dot_width, 0.0, 0.0, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, BottomRight),
        );
        renderer.render_rectangle(
            App::Position::new(
                (available_space.width() - wide_dot_width) / 2.0,
                available_space.height() - dot_width,
            ),
            App::Size::new(wide_dot_width, dot_width),
            screen_clip,
            App::CornerRadius::new(dot_width, dot_width, 0.0, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, BottomCenter),
        );
        renderer.render_rectangle(
            App::Position::new(0.0, available_space.height() - dot_width),
            dot_size,
            screen_clip,
            App::CornerRadius::new(0.0, dot_width, 0.0, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, BottomLeft),
        );
        renderer.render_rectangle(
            App::Position::new(0.0, (available_space.height() - wide_dot_width) / 2.0),
            App::Size::new(dot_width, wide_dot_width),
            screen_clip,
            App::CornerRadius::new(0.0, dot_width, dot_width, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, CenterLeft),
        );
    }
}
