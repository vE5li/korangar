#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::application::{Application, Clip, CornerDiameter, Position, RenderLayer, Size};

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
    initializing: bool,
}

impl<App> Default for Anchor<App>
where
    App: Application,
{
    fn default() -> Self {
        Self {
            anchor_point: AnchorPoint::Center,
            offset: App::Position::new(0.0, 0.0),
            initializing: true,
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
    fn initialized(anchor_point: AnchorPoint, offset: App::Position) -> Self {
        Self {
            anchor_point,
            offset,
            initializing: false,
        }
    }

    pub fn is_initializing(&self) -> bool {
        self.initializing
    }

    pub fn initialize(&mut self, window_size: App::Size, real_size: App::Size) {
        self.offset = App::Position::new(-real_size.width() / 2.0, -window_size.height() / 4.0);
        self.initializing = false;
    }

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

    pub fn update(&mut self, window_space: App::Size, position: App::Position, window_size: App::Size) {
        let center = (
            Anchor::initialized(
                AnchorPoint::Center,
                App::Position::new(
                    position.left() - window_space.width() / 2.0,
                    position.top() - window_space.height() / 2.0,
                ),
            ),
            App::Position::new(
                position.left() - (window_space.width() - window_size.width()) / 2.0,
                position.top() - (window_space.height() - window_size.height()) / 2.0,
            ),
        );

        let top_left = (Anchor::initialized(AnchorPoint::TopLeft, position), position);

        let top_center = (
            Anchor::initialized(
                AnchorPoint::TopCenter,
                App::Position::new(position.left() - window_space.width() / 2.0, position.top()),
            ),
            App::Position::new(
                position.left() - (window_space.width() - window_size.width()) / 2.0,
                position.top(),
            ),
        );

        let top_right = (
            Anchor::initialized(
                AnchorPoint::TopRight,
                App::Position::new(position.left() - window_space.width(), position.top()),
            ),
            App::Position::new(position.left() - window_space.width() + window_size.width(), position.top()),
        );

        let center_right = (
            Anchor::initialized(
                AnchorPoint::CenterRight,
                App::Position::new(
                    position.left() - window_space.width(),
                    position.top() - window_space.height() / 2.0,
                ),
            ),
            App::Position::new(
                position.left() - window_space.width() + window_size.width(),
                position.top() - (window_space.height() - window_size.height()) / 2.0,
            ),
        );

        let bottom_right = (
            Anchor::initialized(
                AnchorPoint::BottomRight,
                App::Position::new(position.left() - window_space.width(), position.top() - window_space.height()),
            ),
            App::Position::new(
                position.left() - window_space.width() + window_size.width(),
                position.top() - window_space.height() + window_size.height(),
            ),
        );

        let bottom_center = (
            Anchor::initialized(
                AnchorPoint::BottomCenter,
                App::Position::new(
                    position.left() - window_space.width() / 2.0,
                    position.top() - window_space.height(),
                ),
            ),
            App::Position::new(
                position.left() - (window_space.width() - window_size.width()) / 2.0,
                position.top() - window_space.height() + window_size.height(),
            ),
        );

        let bottom_left = (
            Anchor::initialized(
                AnchorPoint::BottomLeft,
                App::Position::new(position.left(), position.top() - window_space.height()),
            ),
            App::Position::new(position.left(), position.top() - window_space.height() + window_size.height()),
        );

        let center_left = (
            Anchor::initialized(
                AnchorPoint::CenterLeft,
                App::Position::new(position.left(), position.top() - window_space.height() / 2.0),
            ),
            App::Position::new(
                position.left(),
                position.top() - (window_space.height() - window_size.height()) / 2.0,
            ),
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

    #[allow(clippy::too_many_arguments)]
    pub fn render_window_anchors(
        &self,
        renderer: &App::Renderer,
        anchor_color: App::Color,
        closest_anchor_color: App::Color,
        shadow_color: App::Color,
        shadow_padding: App::ShadowPadding,
        window_position: App::Position,
        window_size: App::Size,
        interface_scaling: f32,
    ) {
        let dot_width = 10.0 * interface_scaling;
        let wide_dot_width = 40.0 * interface_scaling;
        let dot_size = App::Size::new(dot_width, dot_width);
        let screen_clip = App::Clip::unbound();

        renderer.render_rectangle(
            App::Position::new(
                window_position.left() + (window_size.width() - dot_width) / 2.0,
                window_position.top() + (window_size.height() - dot_width) / 2.0,
            ),
            dot_size,
            screen_clip,
            App::CornerDiameter::new(dot_width, dot_width, dot_width, dot_width),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, Center),
            shadow_color,
            shadow_padding,
        );
        renderer.render_rectangle(
            window_position,
            dot_size,
            screen_clip,
            App::CornerDiameter::new(0.0, 0.0, dot_width, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, TopLeft),
            shadow_color,
            shadow_padding,
        );
        renderer.render_rectangle(
            App::Position::new(
                window_position.left() + (window_size.width() - wide_dot_width) / 2.0,
                window_position.top() + 0.0,
            ),
            App::Size::new(wide_dot_width, dot_width),
            screen_clip,
            App::CornerDiameter::new(0.0, 0.0, dot_width, dot_width),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, TopCenter),
            shadow_color,
            shadow_padding,
        );
        renderer.render_rectangle(
            App::Position::new(window_position.left() + window_size.width() - dot_width, window_position.top()),
            dot_size,
            screen_clip,
            App::CornerDiameter::new(0.0, 0.0, 0.0, dot_width),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, TopRight),
            shadow_color,
            shadow_padding,
        );
        renderer.render_rectangle(
            App::Position::new(
                window_position.left() + window_size.width() - dot_width,
                window_position.top() + (window_size.height() - wide_dot_width) / 2.0,
            ),
            App::Size::new(dot_width, wide_dot_width),
            screen_clip,
            App::CornerDiameter::new(dot_width, 0.0, 0.0, dot_width),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, CenterRight),
            shadow_color,
            shadow_padding,
        );
        renderer.render_rectangle(
            App::Position::new(
                window_position.left() + window_size.width() - dot_width,
                window_position.top() + window_size.height() - dot_width,
            ),
            dot_size,
            screen_clip,
            App::CornerDiameter::new(dot_width, 0.0, 0.0, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, BottomRight),
            shadow_color,
            shadow_padding,
        );
        renderer.render_rectangle(
            App::Position::new(
                window_position.left() + (window_size.width() - wide_dot_width) / 2.0,
                window_position.top() + window_size.height() - dot_width,
            ),
            App::Size::new(wide_dot_width, dot_width),
            screen_clip,
            App::CornerDiameter::new(dot_width, dot_width, 0.0, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, BottomCenter),
            shadow_color,
            shadow_padding,
        );
        renderer.render_rectangle(
            App::Position::new(window_position.left(), window_position.top() + window_size.height() - dot_width),
            dot_size,
            screen_clip,
            App::CornerDiameter::new(0.0, dot_width, 0.0, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, BottomLeft),
            shadow_color,
            shadow_padding,
        );
        renderer.render_rectangle(
            App::Position::new(
                window_position.left(),
                window_position.top() + (window_size.height() - wide_dot_width) / 2.0,
            ),
            App::Size::new(dot_width, wide_dot_width),
            screen_clip,
            App::CornerDiameter::new(0.0, dot_width, dot_width, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, CenterLeft),
            shadow_color,
            shadow_padding,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_screen_anchors(
        &self,
        renderer: &App::Renderer,
        anchor_color: App::Color,
        closest_anchor_color: App::Color,
        shadow_color: App::Color,
        shadow_padding: App::ShadowPadding,
        available_space: App::Size,
        interface_scaling: f32,
    ) {
        let dot_width = 10.0 * interface_scaling;
        let wide_dot_width = 60.0 * interface_scaling;
        let dot_size = App::Size::new(dot_width, dot_width);
        let screen_clip = App::Clip::unbound();

        renderer.render_rectangle(
            App::Position::new(
                (available_space.width() - dot_width) / 2.0,
                (available_space.height() - dot_width) / 2.0,
            ),
            dot_size,
            screen_clip,
            App::CornerDiameter::new(dot_width, dot_width, dot_width, dot_width),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, Center),
            shadow_color,
            shadow_padding,
        );
        renderer.render_rectangle(
            App::Position::new(0.0, 0.0),
            dot_size,
            screen_clip,
            App::CornerDiameter::new(0.0, 0.0, dot_width, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, TopLeft),
            shadow_color,
            shadow_padding,
        );
        renderer.render_rectangle(
            App::Position::new((available_space.width() - wide_dot_width) / 2.0, 0.0),
            App::Size::new(wide_dot_width, dot_width),
            screen_clip,
            App::CornerDiameter::new(0.0, 0.0, dot_width, dot_width),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, TopCenter),
            shadow_color,
            shadow_padding,
        );
        renderer.render_rectangle(
            App::Position::new(available_space.width() - dot_width, 0.0),
            dot_size,
            screen_clip,
            App::CornerDiameter::new(0.0, 0.0, 0.0, dot_width),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, TopRight),
            shadow_color,
            shadow_padding,
        );
        renderer.render_rectangle(
            App::Position::new(
                available_space.width() - dot_width,
                (available_space.height() - wide_dot_width) / 2.0,
            ),
            App::Size::new(dot_width, wide_dot_width),
            screen_clip,
            App::CornerDiameter::new(dot_width, 0.0, 0.0, dot_width),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, CenterRight),
            shadow_color,
            shadow_padding,
        );
        renderer.render_rectangle(
            App::Position::new(available_space.width() - dot_width, available_space.height() - dot_width),
            dot_size,
            screen_clip,
            App::CornerDiameter::new(dot_width, 0.0, 0.0, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, BottomRight),
            shadow_color,
            shadow_padding,
        );
        renderer.render_rectangle(
            App::Position::new(
                (available_space.width() - wide_dot_width) / 2.0,
                available_space.height() - dot_width,
            ),
            App::Size::new(wide_dot_width, dot_width),
            screen_clip,
            App::CornerDiameter::new(dot_width, dot_width, 0.0, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, BottomCenter),
            shadow_color,
            shadow_padding,
        );
        renderer.render_rectangle(
            App::Position::new(0.0, available_space.height() - dot_width),
            dot_size,
            screen_clip,
            App::CornerDiameter::new(0.0, dot_width, 0.0, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, BottomLeft),
            shadow_color,
            shadow_padding,
        );
        renderer.render_rectangle(
            App::Position::new(0.0, (available_space.height() - wide_dot_width) / 2.0),
            App::Size::new(dot_width, wide_dot_width),
            screen_clip,
            App::CornerDiameter::new(0.0, dot_width, dot_width, 0.0),
            anchor_color!(self.anchor_point, anchor_color, closest_anchor_color, CenterLeft),
            shadow_color,
            shadow_padding,
        );
    }
}
