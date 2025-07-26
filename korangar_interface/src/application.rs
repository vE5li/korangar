use crate::layout::{ClipLayer, Icon};
use crate::theme::ThemePathGetter;
use crate::window::Anchor;

// TODO: Maybe make Path<State: ?Sized>
pub trait Application: Sized + 'static {
    type Cache: WindowCache<Self>;
    // TODO: Does this default bound really make sense or do we need a different way
    // of deriving PrototyeWindow without specifying the theme in the
    // beginning?
    type ThemeType: Default + Copy;
    type ThemeGetter: ThemePathGetter<Self>;
    type Color: Copy;
    type CornerRadius: CornerRadiusTrait;
    type FontSize: Copy;
    type Renderer: RenderLayer<Self>;

    type Position: PositionTrait;
    type Size: SizeTrait;
    type Clip: ClipTrait;
    type WindowClass: PartialEq + Copy;

    type Event: Clone;

    // fn get_scaling_path() -> impl Path<Self, Scaling>;

    fn set_current_theme_type(theme: Self::ThemeType);
}

pub trait MouseInputModeTrait<App>
where
    App: Application,
{
    fn is_none(&self) -> bool;

    // fn is_self_dragged(&self, element: &dyn Element<App>) -> bool;

    fn is_moving_window(&self, window_index: usize) -> bool;
}

pub trait FontSizeTrait: Copy {
    // fn new(value: f32) -> Self;

    fn get_value(&self) -> f32;
}

pub trait ScalingExt {
    fn scaled(&self, scaling: impl ScalingTrait) -> Self;
}

pub trait RenderLayer<App: Application> {
    type CustomInstruction<'a>;
    type CustomIcon: Clone + Copy;

    fn render_rectangle(
        &self,
        position: App::Position,
        size: App::Size,
        clip: App::Clip,
        corner_radius: App::CornerRadius,
        color: App::Color,
    );

    fn get_text_dimensions(&self, text: &str, font_size: App::FontSize, available_width: f32) -> App::Size;

    fn render_text(&self, text: &str, position: App::Position, clip: App::Clip, color: App::Color, font_size: App::FontSize);

    fn render_icon(&self, position: App::Position, size: App::Size, clip: App::Clip, icon: Icon<App>, color: App::Color);

    fn render_custom(&self, instruction: Self::CustomInstruction<'_>, clip_layers: &[ClipLayer<App>]);
}

pub trait ScalingTrait: Copy {
    fn get_factor(&self) -> f32;
}

pub trait CornerRadiusTrait: Copy {
    fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self;

    fn top_left(&self) -> f32;

    fn top_right(&self) -> f32;

    fn bottom_right(&self) -> f32;

    fn bottom_left(&self) -> f32;
}

// TODO: Rename
pub trait PositionTrait: Copy {
    fn new(left: f32, top: f32) -> Self;

    fn left(&self) -> f32;

    fn top(&self) -> f32;
}

// TODO: Rename
pub trait SizeTrait: Copy {
    fn new(width: f32, height: f32) -> Self;

    fn width(&self) -> f32;

    fn height(&self) -> f32;
}

// TODO: Rename
pub trait ClipTrait: Copy {
    fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self;

    fn unbound() -> Self;

    fn left(&self) -> f32;

    fn right(&self) -> f32;

    fn top(&self) -> f32;

    fn bottom(&self) -> f32;
}

pub trait WindowCache<App>
where
    App: Application,
{
    fn create() -> Self;

    fn register_window(&mut self, window_class: App::WindowClass, anchor: Anchor<App>, size: App::Size);

    fn update_anchor(&mut self, window_class: App::WindowClass, anchor: Anchor<App>);

    fn update_size(&mut self, window_class: App::WindowClass, size: App::Size);

    fn get_window_state(&self, window_class: App::WindowClass) -> Option<(Anchor<App>, App::Size)>;
}
