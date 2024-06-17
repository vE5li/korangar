use serde::{Deserialize, Serialize};

use crate::application::{
    Application, ClipTraitExt, CornerRadiusTrait, InterfaceRenderer, PositionTrait, PositionTraitExt, SizeTrait, SizeTraitExt,
};
use crate::theme::{InterfaceTheme, WindowTheme};

macro_rules! anchor_color {
    ($anchor_point:expr, $theme:expr, $name:ident) => {
        match $anchor_point {
            AnchorPoint::$name => $theme.window().closest_anchor_color(),
            _ => $theme.window().anchor_color(),
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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
            offset: App::Position::zero(),
        }
    }
}

impl<App> Clone for Anchor<App>
where
    App: Application,
{
    fn clone(&self) -> Self {
        Self {
            anchor_point: self.anchor_point,
            offset: self.offset,
        }
    }
}

impl<App> Anchor<App>
where
    App: Application,
{
    pub fn update(&mut self, available_space: App::Size, position: App::Position, size: App::Size) {
        let center = Anchor {
            offset: position
                .offset(size.halved())
                .relative_to(App::Position::from_size(available_space.halved())),
            anchor_point: AnchorPoint::Center,
        };
        let top_left = Anchor {
            offset: position,
            anchor_point: AnchorPoint::TopLeft,
        };
        let top_center = Anchor {
            offset: App::Position::new(position.left() - available_space.shrink(size).halved().width(), position.top()),
            anchor_point: AnchorPoint::TopCenter,
        };
        let top_right = Anchor {
            offset: App::Position::new(position.left() - available_space.shrink(size).width(), position.top()),
            anchor_point: AnchorPoint::TopRight,
        };
        let center_right = Anchor {
            offset: App::Position::new(
                position.left() - available_space.shrink(size).width(),
                position.top() - available_space.shrink(size).halved().height(),
            ),
            anchor_point: AnchorPoint::CenterRight,
        };
        let bottom_right = Anchor {
            offset: position.relative_to(App::Position::from_size(available_space.shrink(size))),
            anchor_point: AnchorPoint::BottomRight,
        };
        let bottom_center = Anchor {
            offset: App::Position::new(
                position.left() - available_space.shrink(size).halved().width(),
                position.top() - available_space.shrink(size).height(),
            ),
            anchor_point: AnchorPoint::BottomCenter,
        };
        let bottom_left = Anchor {
            offset: App::Position::new(position.left(), position.top() - available_space.shrink(size).height()),
            anchor_point: AnchorPoint::BottomLeft,
        };
        let center_left = Anchor {
            offset: App::Position::new(position.left(), position.top() - available_space.shrink(size).halved().height()),
            anchor_point: AnchorPoint::CenterLeft,
        };

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
        .min_by_key(|anchor: &Anchor<App>| anchor.offset.left().abs() as usize + anchor.offset.top().abs() as usize)
        .unwrap();
    }

    pub fn current_position(&self, available_space: App::Size, size: App::Size) -> App::Position {
        match self.anchor_point {
            AnchorPoint::Center => App::Position::from_size(available_space.shrink(size))
                .halved()
                .combined(self.offset),
            AnchorPoint::TopLeft => self.offset,
            AnchorPoint::TopCenter => App::Position::only_left(available_space.width() - size.width())
                .halved()
                .combined(self.offset),
            AnchorPoint::TopRight => App::Position::only_left(available_space.width() - size.width()).combined(self.offset),
            AnchorPoint::CenterRight => App::Position::new(
                available_space.shrink(size).width(),
                available_space.shrink(size).halved().height(),
            )
            .combined(self.offset),
            AnchorPoint::BottomRight => App::Position::from_size(available_space.shrink(size)).combined(self.offset),
            AnchorPoint::BottomCenter => App::Position::new(
                available_space.shrink(size).halved().width(),
                available_space.shrink(size).height(),
            )
            .combined(self.offset),
            AnchorPoint::BottomLeft => App::Position::only_top(available_space.height() - size.height()).combined(self.offset),
            AnchorPoint::CenterLeft => App::Position::only_top(available_space.height() - size.height())
                .halved()
                .combined(self.offset),
        }
    }

    pub(super) fn render_window_anchors(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &App::Renderer,
        theme: &App::Theme,
        window_position: App::Position,
        window_size: App::Size,
    ) {
        let dot_width = 10.0;
        let wide_dot_width = 40.0;
        let dot_size = App::Size::uniform(dot_width);
        let screen_clip = App::Clip::unbound();

        renderer.render_rectangle(
            render_target,
            window_position.offset(window_size.shrink(dot_size).halved()),
            dot_size,
            screen_clip,
            App::CornerRadius::new(dot_width, dot_width, dot_width, dot_width),
            anchor_color!(self.anchor_point, theme, Center),
        );
        renderer.render_rectangle(
            render_target,
            window_position,
            dot_size,
            screen_clip,
            App::CornerRadius::new(0.0, 0.0, dot_width, 0.0),
            anchor_color!(self.anchor_point, theme, TopLeft),
        );
        renderer.render_rectangle(
            render_target,
            window_position.offset(App::Size::only_width(window_size.width() - wide_dot_width).halved()),
            App::Size::new(wide_dot_width, dot_width),
            screen_clip,
            App::CornerRadius::new(0.0, 0.0, dot_width, dot_width),
            anchor_color!(self.anchor_point, theme, TopCenter),
        );
        renderer.render_rectangle(
            render_target,
            window_position.offset(App::Size::only_width(window_size.width() - dot_width)),
            dot_size,
            screen_clip,
            App::CornerRadius::new(0.0, 0.0, 0.0, dot_width),
            anchor_color!(self.anchor_point, theme, TopRight),
        );
        renderer.render_rectangle(
            render_target,
            window_position.offset(App::Size::new(
                window_size.shrink(dot_size).width(),
                window_size.shrink(App::Size::only_height(wide_dot_width)).halved().height(),
            )),
            App::Size::new(dot_width, wide_dot_width),
            screen_clip,
            App::CornerRadius::new(dot_width, 0.0, 0.0, dot_width),
            anchor_color!(self.anchor_point, theme, CenterRight),
        );
        renderer.render_rectangle(
            render_target,
            window_position.offset(window_size.shrink(dot_size)),
            dot_size,
            screen_clip,
            App::CornerRadius::new(dot_width, 0.0, 0.0, 0.0),
            anchor_color!(self.anchor_point, theme, BottomRight),
        );
        renderer.render_rectangle(
            render_target,
            window_position.offset(App::Size::new(
                window_size.shrink(App::Size::only_width(wide_dot_width)).halved().width(),
                window_size.shrink(dot_size).height(),
            )),
            App::Size::new(wide_dot_width, dot_width),
            screen_clip,
            App::CornerRadius::new(dot_width, dot_width, 0.0, 0.0),
            anchor_color!(self.anchor_point, theme, BottomCenter),
        );
        renderer.render_rectangle(
            render_target,
            window_position.offset(App::Size::only_height(window_size.height() - dot_width)),
            dot_size,
            screen_clip,
            App::CornerRadius::new(0.0, dot_width, 0.0, 0.0),
            anchor_color!(self.anchor_point, theme, BottomLeft),
        );
        renderer.render_rectangle(
            render_target,
            window_position.offset(App::Size::only_height(window_size.height() - wide_dot_width).halved()),
            App::Size::new(dot_width, wide_dot_width),
            screen_clip,
            App::CornerRadius::new(0.0, dot_width, dot_width, 0.0),
            anchor_color!(self.anchor_point, theme, CenterLeft),
        );
    }

    pub(super) fn render_screen_anchors(
        &self,
        render_target: &mut <App::Renderer as InterfaceRenderer<App>>::Target,
        renderer: &App::Renderer,
        theme: &App::Theme,
        available_space: App::Size,
    ) {
        let dot_width = 10.0;
        let wide_dot_width = 60.0;
        let dot_size = App::Size::uniform(dot_width);
        let screen_clip = App::Clip::unbound();

        renderer.render_rectangle(
            render_target,
            App::Position::from_size(available_space.shrink(dot_size).halved()),
            dot_size,
            screen_clip,
            App::CornerRadius::new(dot_width, dot_width, dot_width, dot_width),
            anchor_color!(self.anchor_point, theme, Center),
        );
        renderer.render_rectangle(
            render_target,
            App::Position::zero(),
            dot_size,
            screen_clip,
            App::CornerRadius::new(0.0, 0.0, dot_width, 0.0),
            anchor_color!(self.anchor_point, theme, TopLeft),
        );
        renderer.render_rectangle(
            render_target,
            App::Position::only_left(available_space.width() - wide_dot_width).halved(),
            App::Size::new(wide_dot_width, dot_width),
            screen_clip,
            App::CornerRadius::new(0.0, 0.0, dot_width, dot_width),
            anchor_color!(self.anchor_point, theme, TopCenter),
        );
        renderer.render_rectangle(
            render_target,
            App::Position::only_left(available_space.width() - dot_width),
            dot_size,
            screen_clip,
            App::CornerRadius::new(0.0, 0.0, 0.0, dot_width),
            anchor_color!(self.anchor_point, theme, TopRight),
        );
        renderer.render_rectangle(
            render_target,
            App::Position::new(
                available_space.shrink(dot_size).width(),
                available_space.shrink(App::Size::only_height(wide_dot_width)).halved().height(),
            ),
            App::Size::new(dot_width, wide_dot_width),
            screen_clip,
            App::CornerRadius::new(dot_width, 0.0, 0.0, dot_width),
            anchor_color!(self.anchor_point, theme, CenterRight),
        );
        renderer.render_rectangle(
            render_target,
            App::Position::from_size(available_space.shrink(dot_size)),
            dot_size,
            screen_clip,
            App::CornerRadius::new(dot_width, 0.0, 0.0, 0.0),
            anchor_color!(self.anchor_point, theme, BottomRight),
        );
        renderer.render_rectangle(
            render_target,
            App::Position::new(
                available_space.shrink(App::Size::only_width(wide_dot_width)).halved().width(),
                available_space.shrink(dot_size).height(),
            ),
            App::Size::new(wide_dot_width, dot_width),
            screen_clip,
            App::CornerRadius::new(dot_width, dot_width, 0.0, 0.0),
            anchor_color!(self.anchor_point, theme, BottomCenter),
        );
        renderer.render_rectangle(
            render_target,
            App::Position::only_top(available_space.height() - dot_width),
            dot_size,
            screen_clip,
            App::CornerRadius::new(0.0, dot_width, 0.0, 0.0),
            anchor_color!(self.anchor_point, theme, BottomLeft),
        );
        renderer.render_rectangle(
            render_target,
            App::Position::only_top(available_space.height() - wide_dot_width).halved(),
            App::Size::new(dot_width, wide_dot_width),
            screen_clip,
            App::CornerRadius::new(0.0, dot_width, dot_width, 0.0),
            anchor_color!(self.anchor_point, theme, CenterLeft),
        );
    }
}
