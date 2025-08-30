use super::area::{Area, PartialArea};
use crate::application::{Application, TextLayouter};
use crate::prelude::HorizontalAlignment;

pub struct Resolver<'a, App: Application> {
    available_area: PartialArea,
    used_height: f32,
    gaps: f32,
    text_layouter: &'a App::TextLayouter,
}

impl<App> Clone for Resolver<'_, App>
where
    App: Application,
{
    fn clone(&self) -> Self {
        Self {
            available_area: self.available_area,
            used_height: self.used_height,
            gaps: self.gaps,
            text_layouter: self.text_layouter,
        }
    }
}

impl<'a, App> Resolver<'a, App>
where
    App: Application,
{
    pub fn new(available_area: impl Into<PartialArea>, gaps: f32, text_layouter: &'a App::TextLayouter) -> Self {
        Self {
            available_area: available_area.into(),
            used_height: 0.0,
            gaps,
            text_layouter,
        }
    }

    fn push_gaps(&mut self) {
        if self.used_height > 0.0 {
            self.available_area.top += self.gaps;
            self.used_height += self.gaps;

            if let Some(available_height) = &mut self.available_area.height {
                *available_height -= self.gaps;
            }
        }
    }

    pub fn push_available_area(&mut self) -> PartialArea {
        self.push_gaps();

        self.available_area
    }

    pub fn with_height(&mut self, height: f32) -> Area {
        self.push_gaps();

        let returned = Area {
            left: self.available_area.left,
            top: self.available_area.top,
            width: self.available_area.width,
            height,
        };

        self.available_area.top += height;
        self.used_height += height;

        if let Some(available_height) = &mut self.available_area.height {
            *available_height -= height;
        }

        returned
    }

    pub fn get_text_dimensions(
        &self,
        text: &str,
        default_color: App::Color,
        highlight_color: App::Color,
        font_size: App::FontSize,
        horizontal_alignment: HorizontalAlignment,
        overflow_behavior: App::OverflowBehavior,
    ) -> (App::Size, App::FontSize) {
        let offset = match horizontal_alignment {
            HorizontalAlignment::Left { offset, border } => offset + border,
            HorizontalAlignment::Center { offset, border } => offset + border,
            HorizontalAlignment::Right { offset, border } => offset + border,
        };

        self.text_layouter.get_text_dimensions(
            text,
            default_color,
            highlight_color,
            font_size,
            self.available_area.width - offset,
            overflow_behavior,
        )
    }

    pub fn get_text_layouter(&self) -> &App::TextLayouter {
        self.text_layouter
    }

    pub fn push_top(&mut self, height: f32) {
        self.push_gaps();

        self.available_area.top += height;
        self.used_height += height;

        if let Some(available_height) = &mut self.available_area.height {
            *available_height -= height;
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("derived"))]
    pub fn with_derived<L>(&mut self, gaps: f32, border: f32, f: impl FnOnce(&mut Resolver<'a, App>) -> L) -> (Area, L) {
        self.push_gaps();

        let mut inner = Resolver {
            available_area: PartialArea {
                left: self.available_area.left + border,
                top: self.available_area.top + border,
                width: self.available_area.width - border * 2.0,
                height: self.available_area.height.map(|height| height - border * 2.0),
            },
            used_height: 0.0,
            gaps,
            text_layouter: self.text_layouter,
        };

        let layout_info = f(&mut inner);

        let returned = Area {
            left: self.available_area.left,
            top: self.available_area.top,
            width: self.available_area.width,
            height: inner.used_height + border * 2.0,
        };

        self.available_area.top += returned.height;
        self.used_height += returned.height;

        if let Some(available_height) = &mut self.available_area.height {
            *available_height -= returned.height;
        }

        (returned, layout_info)
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("derived"))]
    pub fn with_derived_borderless<L>(
        &mut self,
        gaps: f32,
        border: f32,
        once_gap: f32,
        f: impl FnOnce(&mut Resolver<'a, App>) -> L,
    ) -> (Area, L) {
        self.push_gaps();

        let mut inner = Resolver {
            available_area: PartialArea {
                left: self.available_area.left + border,
                top: self.available_area.top + once_gap,
                width: self.available_area.width - border * 2.0,
                height: self.available_area.height.map(|height| height - border - once_gap),
            },
            used_height: 0.0,
            gaps,
            text_layouter: self.text_layouter,
        };

        let layout_info = f(&mut inner);

        let returned = Area {
            left: self.available_area.left,
            top: self.available_area.top,
            width: self.available_area.width,
            height: inner.used_height + border + once_gap,
        };

        self.available_area.top += returned.height;
        self.used_height += returned.height;

        if let Some(available_height) = &mut self.available_area.height {
            *available_height -= returned.height;
        }

        (returned, layout_info)
    }

    pub fn with_derived_scrolled<L>(&mut self, scroll: f32, f: impl FnOnce(&mut Resolver<'a, App>) -> L) -> (Area, f32, L) {
        self.push_gaps();

        let mut inner = Resolver {
            available_area: PartialArea {
                left: self.available_area.left,
                top: self.available_area.top - scroll,
                width: self.available_area.width,
                height: None,
            },
            used_height: 0.0,
            gaps: self.gaps,
            text_layouter: self.text_layouter,
        };

        let layout_info = f(&mut inner);

        let children_height = inner.used_height;
        let height = self
            .available_area
            .height
            .expect("attempted to get height from an unbound resolver");

        let returned = Area {
            left: self.available_area.left,
            top: self.available_area.top,
            width: self.available_area.width,
            height,
        };

        self.available_area.top += returned.height;
        self.used_height += returned.height;

        if let Some(available_height) = &mut self.available_area.height {
            *available_height -= returned.height;
        }

        (returned, children_height, layout_info)
    }

    pub fn get_used_height(&self) -> f32 {
        self.used_height
    }

    pub fn commit_used_height(&mut self, used_height: f32) {
        self.available_area.top += used_height;
        self.used_height += used_height;

        if let Some(available_height) = &mut self.available_area.height {
            *available_height -= used_height;
        }
    }
}

pub trait ResolverSet<'a, App: Application> {
    fn with_index<C>(&mut self, index: usize, f: impl FnMut(&mut Resolver<'a, App>) -> C) -> C;
}

impl<'a, App> ResolverSet<'a, App> for &mut Resolver<'a, App>
where
    App: Application,
{
    fn with_index<C>(&mut self, _: usize, mut f: impl FnMut(&mut Resolver<'a, App>) -> C) -> C {
        f(*self)
    }
}
