use std::alloc::Allocator;
use std::any::Any;
use std::array;
use std::cmp::Ordering;
use std::fmt::Display;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use interface_components::collapsable;
use rust_state::{ArrayLookupExt, Context, ManuallyAssertExt, OptionExt, Path, Selector, VecIndexExt};

use super::store::ElementStoreMut;
use super::{Element, ElementSet};
use crate::application::Application;
use crate::components::text_box::DefaultHandler;
use crate::element::BaseLayoutInfo;
use crate::element::store::ElementStore;
use crate::event::ClickHandler;
use crate::layout::area::Area;
use crate::layout::tooltip::TooltipExt;
use crate::layout::{Icon, MouseButton, Resolver, ResolverSet, WindowLayout};
use crate::prelude::CollapsableThemePathExt;
use crate::theme::{ThemePathGetter, theme};

pub trait StateElement<App: Application> {
    type Return<P>: Element<App, LayoutInfo = Self::LayoutInfo>
    where
        P: Path<App, Self>;

    type LayoutInfo;

    fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
    where
        P: Path<App, Self>;

    type ReturnMut<P>: Element<App, LayoutInfo = Self::LayoutInfoMut>
    where
        P: Path<App, Self>;

    type LayoutInfoMut;

    fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
    where
        P: Path<App, Self>;
}

pub trait ElementDisplay: PartialEq + Clone + 'static {
    fn element_display(&self) -> String;
}

// workaround for not having negative trait bounds or better specialization
auto trait NoElementDisplay {}
impl !NoElementDisplay for bool {}
impl !NoElementDisplay for f32 {}
#[cfg(feature = "cgmath")]
impl<T> !NoElementDisplay for cgmath::Point3<T> {}
#[cfg(feature = "cgmath")]
impl<T> !NoElementDisplay for cgmath::Vector2<T> {}
#[cfg(feature = "cgmath")]
impl<T> !NoElementDisplay for cgmath::Vector3<T> {}
#[cfg(feature = "cgmath")]
impl<T> !NoElementDisplay for cgmath::Vector4<T> {}
#[cfg(feature = "cgmath")]
impl<T> !NoElementDisplay for cgmath::Quaternion<T> {}
#[cfg(feature = "cgmath")]
impl<T> !NoElementDisplay for cgmath::Rad<T> {}

impl<T> ElementDisplay for T
where
    T: PartialEq + Clone + Display + NoElementDisplay + 'static,
{
    fn element_display(&self) -> String {
        self.to_string()
    }
}

impl ElementDisplay for f32 {
    fn element_display(&self) -> String {
        format!("{self:.1}")
    }
}

#[cfg(feature = "cgmath")]
impl<T: ElementDisplay> ElementDisplay for cgmath::Point3<T> {
    fn element_display(&self) -> String {
        format!(
            "{}, {}, {}",
            self.x.element_display(),
            self.y.element_display(),
            self.z.element_display()
        )
    }
}

#[cfg(feature = "cgmath")]
impl<T: ElementDisplay> ElementDisplay for cgmath::Vector2<T> {
    fn element_display(&self) -> String {
        format!("{}, {}", self.x.element_display(), self.y.element_display())
    }
}

#[cfg(feature = "cgmath")]
impl<T: ElementDisplay> ElementDisplay for cgmath::Vector3<T> {
    fn element_display(&self) -> String {
        format!(
            "{}, {}, {}",
            self.x.element_display(),
            self.y.element_display(),
            self.z.element_display()
        )
    }
}

#[cfg(feature = "cgmath")]
impl<T: ElementDisplay> ElementDisplay for cgmath::Vector4<T> {
    fn element_display(&self) -> String {
        format!(
            "{}, {}, {}, {}",
            self.x.element_display(),
            self.y.element_display(),
            self.z.element_display(),
            self.w.element_display()
        )
    }
}

#[cfg(feature = "cgmath")]
impl<T: ElementDisplay> ElementDisplay for cgmath::Quaternion<T> {
    fn element_display(&self) -> String {
        format!(
            "{:.1}, {:.1}, {:.1} - {:.1}",
            self.v.x.element_display(),
            self.v.y.element_display(),
            self.v.z.element_display(),
            self.s.element_display()
        )
    }
}

#[cfg(feature = "cgmath")]
impl<T: ElementDisplay> ElementDisplay for cgmath::Rad<T> {
    fn element_display(&self) -> String {
        self.0.element_display()
    }
}

// workaround for not having negative trait bounds or better specialization
auto trait NoPrototype {}
impl<T: ?Sized, A: Allocator> !NoPrototype for Arc<T, A> {}
impl<T> !NoPrototype for Option<T> {}
impl<T, const N: usize> !NoPrototype for [T; N] {}
impl<T> !NoPrototype for &[T] {}
impl<T: ?Sized, A: Allocator> !NoPrototype for Vec<T, A> {}
impl<T: ?Sized, A: Allocator> !NoPrototype for Rc<T, A> {}

impl NoPrototype for &str {}

impl<App, T> StateElement<App> for T
where
    App: Application,
    T: ElementDisplay + NoPrototype,
{
    type LayoutInfo = impl Any;
    type LayoutInfoMut = impl Any;
    type Return<P>
        = impl Element<App, LayoutInfo = Self::LayoutInfo>
    where
        P: Path<App, Self>;
    type ReturnMut<P>
        = impl Element<App, LayoutInfo = Self::LayoutInfoMut>
    where
        P: Path<App, Self>;

    fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
    where
        P: Path<App, Self>,
    {
        use korangar_interface::prelude::*;

        split! {
            children: (
                text! {
                    text: name,
                },
                field! {
                    text: ElementDisplaySelector::new(self_path),
                },
            ),
        }
    }

    fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
    where
        P: Path<App, Self>,
    {
        Self::to_element(self_path, name)
    }
}

impl<App> StateElement<App> for bool
where
    App: Application,
{
    type LayoutInfo = impl Any;
    type LayoutInfoMut = impl Any;
    type Return<P>
        = impl Element<App, LayoutInfo = Self::LayoutInfo>
    where
        P: Path<App, Self>;
    type ReturnMut<P>
        = impl Element<App, LayoutInfo = Self::LayoutInfoMut>
    where
        P: Path<App, Self>;

    fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
    where
        P: Path<App, Self>,
    {
        use korangar_interface::prelude::*;

        struct BoolSelector<P>(P);

        impl<App, P> Selector<App, &'static str> for BoolSelector<P>
        where
            P: Path<App, bool>,
        {
            fn select<'a>(&'a self, state: &'a App) -> Option<&'a &'static str> {
                // SAFETY:
                // It is safe to unwrap here because of the bound.
                match *self.0.follow(state).unwrap() {
                    true => Some(&"True"),
                    false => Some(&"False"),
                }
            }
        }

        split! {
            children: (
                text! {
                    text: name,
                },
                field! {
                    text: BoolSelector(self_path),
                },
            ),
        }
    }

    fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
    where
        P: Path<App, Self>,
    {
        use korangar_interface::prelude::*;

        state_button! {
            text: name,
            state: self_path,
            event: Toggle(self_path),
        }
    }
}

impl<App> StateElement<App> for String
where
    App: Application,
{
    type LayoutInfo = impl Any;
    type LayoutInfoMut = impl Any;
    type Return<P>
        = impl Element<App, LayoutInfo = Self::LayoutInfo>
    where
        P: Path<App, Self>;
    type ReturnMut<P>
        = impl Element<App, LayoutInfo = Self::LayoutInfoMut>
    where
        P: Path<App, Self>;

    fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
    where
        P: Path<App, Self>,
    {
        use korangar_interface::prelude::*;

        split! {
            children: (
                text! {
                    text: name,
                },
                field! {
                    text: self_path,
                },
            ),
        }
    }

    fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
    where
        P: Path<App, Self>,
    {
        use korangar_interface::prelude::*;

        struct PrivateFocusId;

        let action = move |_: &Context<App>, queue: &mut EventQueue<App>| {
            queue.queue(Event::Unfocus);
        };

        split! {
            children: (
                text! {
                    text: name,
                },
                text_box! {
                    ghost_text: "Empty string",
                    state: self_path,
                    input_handler: DefaultHandler::<_, _, { usize::MAX }>::new(self_path, action),
                    focus_id: PrivateFocusId,
                },
            ),
        }
    }
}

struct ArcPath<State, P, T> {
    path: P,
    _marker: PhantomData<(State, T)>,
}

impl<State, P, T> ArcPath<State, P, T> {
    fn new(path: P) -> Self {
        Self {
            path,
            _marker: PhantomData,
        }
    }
}

impl<State, P, T> Clone for ArcPath<State, P, T>
where
    P: Path<State, Arc<T>>,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<State, P, T> Copy for ArcPath<State, P, T> where P: Path<State, Arc<T>> {}

impl<State, P, T> Selector<State, T> for ArcPath<State, P, T>
where
    State: 'static,
    P: Path<State, Arc<T>>,
    T: 'static,
{
    fn select<'a>(&'a self, state: &'a State) -> Option<&'a T> {
        self.follow(state)
    }
}

impl<State, P, T> Path<State, T> for ArcPath<State, P, T>
where
    State: 'static,
    P: Path<State, Arc<T>>,
    T: 'static,
{
    fn follow<'a>(&self, state: &'a State) -> Option<&'a T> {
        self.path.follow(state).map(AsRef::as_ref)
    }

    fn follow_mut<'a>(&self, _: &'a mut State) -> Option<&'a mut T> {
        unimplemented!()
    }
}

impl<App, T> StateElement<App> for Arc<T>
where
    App: Application,
    T: StateElement<App> + 'static,
{
    type LayoutInfo = impl Any;
    type LayoutInfoMut = impl Any;
    type Return<P>
        = impl Element<App, LayoutInfo = Self::LayoutInfo>
    where
        P: Path<App, Self>;
    type ReturnMut<P>
        = impl Element<App, LayoutInfo = Self::LayoutInfoMut>
    where
        P: Path<App, Self>;

    fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
    where
        P: Path<App, Self>,
    {
        T::to_element(ArcPath::new(self_path), name)
    }

    fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
    where
        P: Path<App, Self>,
    {
        T::to_element(ArcPath::new(self_path), name)
    }
}

enum OptionLayoutInfo<N, S> {
    None(N),
    Some(S),
}

struct OptionWrapper<App, O, P, E, T>
where
    App: Application,
    O: Path<App, Option<T>>,
    P: Path<App, T>,
    T: StateElement<App> + 'static,
{
    name: Option<String>,
    option_path: O,
    inner_path: P,
    none_element: E,
    element: Option<T::ReturnMut<P>>,
    _marker: PhantomData<App>,
}

impl<App, O, P, E, T> Element<App> for OptionWrapper<App, O, P, E, T>
where
    App: Application,
    O: Path<App, Option<T>>,
    P: Path<App, T>,
    E: Element<App>,
    T: StateElement<App> + 'static,
{
    type LayoutInfo = OptionLayoutInfo<E::LayoutInfo, T::LayoutInfoMut>;

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        store: ElementStoreMut<'_>,
        resolver: &mut Resolver<'_, App>,
    ) -> Self::LayoutInfo {
        if state.get(&self.option_path).is_some() {
            let element = self
                .element
                .get_or_insert_with(|| T::to_element_mut(self.inner_path, self.name.take().unwrap()));

            OptionLayoutInfo::Some(element.create_layout_info(state, store, resolver))
        } else {
            OptionLayoutInfo::None(self.none_element.create_layout_info(state, store, resolver))
        }
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        match layout_info {
            OptionLayoutInfo::None(layout_info) => self.none_element.lay_out(state, store, layout_info, layout),
            OptionLayoutInfo::Some(layout_info) => self.element.as_ref().unwrap().lay_out(state, store, layout_info, layout),
        }
    }
}

impl<App, T> StateElement<App> for Option<T>
where
    App: Application,
    T: StateElement<App> + 'static,
{
    type LayoutInfo = impl Any;
    type LayoutInfoMut = impl Any;
    type Return<P>
        = impl Element<App, LayoutInfo = Self::LayoutInfo>
    where
        P: Path<App, Self>;
    type ReturnMut<P>
        = impl Element<App, LayoutInfo = Self::LayoutInfoMut>
    where
        P: Path<App, Self>;

    fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
    where
        P: Path<App, Self>,
    {
        use korangar_interface::prelude::*;

        OptionWrapper {
            name: Some(name.clone()),
            option_path: self_path,
            inner_path: self_path.unwrapped().manually_asserted(),
            none_element: split! {
                children: (
                    text! {
                        text: name,
                    },
                    field! {
                        text: "None",
                    }
                ),
            },
            element: None,
            _marker: PhantomData,
        }
    }

    fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
    where
        P: Path<App, Self>,
    {
        use korangar_interface::prelude::*;

        OptionWrapper {
            name: Some(name.clone()),
            option_path: self_path,
            inner_path: self_path.unwrapped().manually_asserted(),
            none_element: split! {
                children: (
                    text! {
                        text: name,
                    },
                    field! {
                        text: "None",
                    }
                ),
            },
            element: None,
            _marker: PhantomData,
        }
    }
}

impl<App, T, const SIZE: usize> StateElement<App> for [T; SIZE]
where
    App: Application,
    T: StateElement<App> + 'static,
{
    type LayoutInfo = impl Any;
    type LayoutInfoMut = impl Any;
    type Return<P>
        = impl Element<App, LayoutInfo = Self::LayoutInfo>
    where
        P: Path<App, Self>;
    type ReturnMut<P>
        = impl Element<App, LayoutInfo = Self::LayoutInfoMut>
    where
        P: Path<App, Self>;

    fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
    where
        P: Path<App, Self>,
    {
        let elements: [impl Element<App>; SIZE] = array::from_fn(|index| {
            let item_path = self_path.array_index(index).manually_asserted();
            T::to_element(item_path, index.to_string())
        });

        collapsable! { text: name, children: elements }
    }

    fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
    where
        P: Path<App, Self>,
    {
        let elements: [impl Element<App>; SIZE] = array::from_fn(|index| {
            let item_path = self_path.array_index(index).manually_asserted();
            T::to_element_mut(item_path, index.to_string())
        });

        collapsable! { text: name, children: elements }
    }
}

struct VecWrapper<App, T, P>
where
    App: Application,
    T: StateElement<App>,
{
    self_path: P,
    item_boxes: Vec<Box<dyn Element<App, LayoutInfo = <T as StateElement<App>>::LayoutInfoMut>>>,
    _marker: PhantomData<T>,
}

// NOTE: We implement `ElementSet` rather than `Element` so that the collapsable
// can check if the number of elements is larger than zero. That way empty
// `collapsable`s will be rendered correctly.
impl<App, T, P> ElementSet<App> for VecWrapper<App, T, P>
where
    App: Application,
    T: StateElement<App> + 'static,
    P: Path<App, Vec<T>>,
{
    // TODO: Refactor to not have to re-allocate this every frame.
    type LayoutInfo = Vec<T::LayoutInfoMut>;

    fn get_element_count(&self, state: &Context<App>) -> usize {
        state.get(&self.self_path).len()
    }

    fn create_layout_info(
        &mut self,
        state: &Context<App>,
        mut store: ElementStoreMut<'_>,
        mut resolver_set: impl ResolverSet<'_, App>,
    ) -> Self::LayoutInfo {
        let vector = state.get(&self.self_path);

        match self.item_boxes.len().cmp(&vector.len()) {
            Ordering::Greater => {
                // Delete excess elements.
                self.item_boxes.truncate(vector.len());
            }
            Ordering::Less => {
                // Add new elements.
                for index in self.item_boxes.len()..vector.len() {
                    self.item_boxes.push({
                        let item_path = self.self_path.index(index).manually_asserted();
                        let item_element = StateElement::to_element_mut(item_path, index.to_string());
                        let item_box: Box<dyn Element<App, LayoutInfo = <T as StateElement<App>>::LayoutInfoMut>> = Box::new(item_element);
                        item_box
                    });
                }
            }
            Ordering::Equal => {}
        }

        resolver_set.with_index(0, |resolver| {
            let (_area, layout_info) = resolver.with_derived(2.0, 4.0, |resolver| {
                self.item_boxes
                    .iter_mut()
                    .enumerate()
                    .map(|(index, item_box)| item_box.create_layout_info(state, store.child_store(index as u64), resolver))
                    .collect()
            });

            layout_info
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a Context<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        for (index, item_box) in self.item_boxes.iter().enumerate() {
            item_box.lay_out(state, store.child_store(index as u64), &layout_info[index], layout);
        }
    }
}

// NOTE: This is generally not recommended if the type can be freely defined
// since the element store for each element is bound to the index, so changes in
// the vector might result in unexpected UI behavior. E.g. removing the first
// item of a Vec might result in the item in the second position being expanded,
// even though the now-first element should be.
// Furthermore this might also result in crashes if different instances of `T`
// require different store data (for example when using trait objects). So use
// with care!
impl<App, T> StateElement<App> for Vec<T>
where
    App: Application,
    T: StateElement<App> + 'static,
{
    type LayoutInfo = impl Any;
    type LayoutInfoMut = impl Any;
    type Return<P>
        = impl Element<App, LayoutInfo = Self::LayoutInfo>
    where
        P: Path<App, Self>;
    type ReturnMut<P>
        = impl Element<App, LayoutInfo = Self::LayoutInfoMut>
    where
        P: Path<App, Self>;

    fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
    where
        P: Path<App, Self>,
    {
        collapsable! {
            text: name,
            children: VecWrapper {
                self_path,
                item_boxes: Vec::new(),
                _marker: PhantomData,
            },
        }
    }

    fn to_element_mut<P>(self_path: P, name: String) -> Self::ReturnMut<P>
    where
        P: Path<App, Self>,
    {
        struct ClearButton<A> {
            pub event: A,
        }

        impl<App, A> Element<App> for ClearButton<A>
        where
            App: Application,
            A: ClickHandler<App> + 'static,
        {
            type LayoutInfo = BaseLayoutInfo;

            fn create_layout_info(
                &mut self,
                state: &Context<App>,
                _: ElementStoreMut<'_>,
                resolver: &mut Resolver<'_, App>,
            ) -> Self::LayoutInfo {
                let height = *state.get(&theme().collapsable().title_height());
                let mut area = resolver.with_height(height);

                // This is making the button square and sit to the right of the title.
                // It's a bit hacky but it does the job for now.
                area.left += area.width - area.height;
                area.width = area.height;

                Self::LayoutInfo { area }
            }

            fn lay_out<'a>(
                &'a self,
                state: &'a Context<App>,
                _: ElementStore<'a>,
                layout_info: &'a Self::LayoutInfo,
                layout: &mut WindowLayout<'a, App>,
            ) {
                let is_hoverered = layout_info.area.check().run(layout);

                if is_hoverered {
                    layout.add_click_area(layout_info.area, MouseButton::Left, &self.event);

                    struct ClearTooltip;
                    layout.add_tooltip("Clear the entire vector", ClearTooltip.tooltip_id());
                }

                // TODO: Don't hardcode distance.
                let icon_area = Area {
                    left: layout_info.area.left + 4.0,
                    top: layout_info.area.top + 4.0,
                    width: layout_info.area.width - 8.0,
                    height: layout_info.area.height - 8.0,
                };

                let icon_color = match is_hoverered {
                    true => *state.get(&theme().collapsable().hovered_foreground_color()),
                    false => *state.get(&theme().collapsable().foreground_color()),
                };

                layout.add_icon(icon_area, Icon::TrashCan, icon_color);
            }
        }

        collapsable! {
            text: name,
            children: VecWrapper {
                self_path,
                item_boxes: Vec::new(),
                _marker: PhantomData,
            },
            extra_elements: (
                ClearButton {
                    event: move |state: &rust_state::Context<App>, _: &mut korangar_interface::event::EventQueue<App>| {
                        state.update_value_with(self_path, |vector| vector.clear());
                    }
                },
            ),
        }
    }
}
