use super::area::{Area, PartialArea};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HeightBound {
    Unbound,
    WithMax,
}

#[derive(Clone)]
pub struct Resolver {
    available_area: PartialArea,
    used_height: f32,
    gaps: f32,
}

impl Resolver {
    pub fn new(available_area: Area, gaps: f32) -> Self {
        Self {
            available_area: available_area.into(),
            used_height: 0.0,
            gaps,
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

    pub fn push_top(&mut self, height: f32) {
        self.push_gaps();

        self.available_area.top += height;
        self.used_height += height;

        if let Some(available_height) = &mut self.available_area.height {
            *available_height -= height;
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("derived"))]
    pub fn with_derived<L>(&mut self, gaps: f32, border: f32, f: impl FnOnce(&mut Resolver) -> L) -> (Area, L) {
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

    pub fn with_derived_scrolled<L>(
        &mut self,
        scroll: f32,
        height_bound: HeightBound,
        f: impl FnOnce(&mut Resolver) -> L,
    ) -> (Area, f32, L) {
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
        };

        let layout_info = f(&mut inner);

        let children_height = inner.used_height;
        let height = match height_bound {
            HeightBound::Unbound => children_height,
            HeightBound::WithMax => children_height.min(
                self.available_area
                    .height
                    .expect("attempted to get height from an unbound resolver"),
            ),
        };

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

    pub fn with_derived_custom<L>(&mut self, available_area: PartialArea, f: impl FnOnce(&mut Resolver) -> L) -> L {
        let mut inner = Resolver {
            available_area,
            used_height: 0.0,
            gaps: self.gaps,
        };

        let layout_info = f(&mut inner);

        let delta = inner.available_area.top - self.available_area.top;
        if delta > 0.0 {
            self.available_area.top = inner.available_area.top;
            self.used_height += delta;
        }

        // TODO: Really bad. Shouldn't unwrap probably
        // self.available_area.height = self
        //     .available_area
        //     .height
        //     .map(|height| height.min(other.available_area.height.unwrap()))
        //     .or(other.available_area.height);

        layout_info
    }
}
