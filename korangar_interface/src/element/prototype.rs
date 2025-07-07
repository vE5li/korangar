use std::any::Any;
use std::array;
use std::cell::UnsafeCell;
use std::cmp::Ordering;
use std::fmt::Display;
use std::marker::PhantomData;
use std::rc::Rc;

use interface_components::{button, collapsable};
use rust_state::{ArrayLookupExt, Context, ManuallyAssertExt, OptionExt, Path};

use super::Element;
use crate::application::Appli;
use crate::element::id::ElementIdGenerator;
use crate::element::store::ElementStore;
use crate::element::{DefaultLayouted, ElementSet, ResolverSet};
use crate::event::EventQueue;
use crate::layout::area::Area;
use crate::layout::{Layout, Resolver};
use crate::theme::theme;

// TODO: Rename this to StateElement
pub trait PrototypeElement<App: Appli> {
    type Return<P>: Element<App, Layouted = Self::Layouted>
    where
        P: Path<App, Self>;

    type Layouted;

    fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
    where
        P: Path<App, Self>;

    // TODO: Add `to_element_mut`
}

pub trait ElementDisplay: PartialEq + Clone + 'static {
    fn element_display(&self) -> String;
}

// workaround for not having negative trait bounds or better specialization
auto trait NoElementDisplay {}
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
impl<T> !NoPrototype for std::sync::Arc<T> {}
impl<T> !NoPrototype for Option<T> {}
impl<T, const N: usize> !NoPrototype for [T; N] {}
impl<T> !NoPrototype for &[T] {}
impl<T> !NoPrototype for Vec<T> {}
impl<T> !NoPrototype for Rc<T> {}

impl NoPrototype for &str {}
impl NoPrototype for String {}

impl<App, T> PrototypeElement<App> for T
where
    App: Appli,
    T: ElementDisplay + NoPrototype,
{
    type Layouted = impl Any;
    type Return<P>
        = impl Element<App, Layouted = Self::Layouted>
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
                text! {
                    text: ElementDisplaySelector::new(self_path),
                },
            ),
        }
    }
}

impl<App, T> PrototypeElement<App> for std::sync::Arc<T>
where
    App: Appli,
    T: PrototypeElement<App>,
{
    type Layouted = impl Any;
    type Return<P>
        = impl Element<App, Layouted = Self::Layouted>
    where
        P: Path<App, Self>;

    fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
    where
        P: Path<App, Self>,
    {
        use korangar_interface::prelude::*;

        button! {
            text: name,
            event: |_: &rust_state::Context<App>, _: &mut EventQueue<App>| {
                println!("Just a dummy for now");
            },
        }
    }
}

impl<App, T> PrototypeElement<App> for Option<T>
where
    App: Appli,
    T: PrototypeElement<App> + 'static,
{
    type Layouted = impl Any;
    type Return<P>
        = impl Element<App, Layouted = Self::Layouted>
    where
        P: Path<App, Self>;

    fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
    where
        P: Path<App, Self>,
    {
        use korangar_interface::prelude::*;

        enum InnerLayouted<N, S> {
            None(N),
            Some(S),
        }

        struct Inner<App, O, P, E, T>
        where
            App: Appli,
            O: Path<App, Option<T>>,
            P: Path<App, T>,
            T: PrototypeElement<App> + 'static,
        {
            name: Option<String>,
            option_path: O,
            inner_path: P,
            none_element: E,
            element: Option<T::Return<P>>,
            _marker: PhantomData<App>,
        }

        impl<App, O, P, E, T> Element<App> for Inner<App, O, P, E, T>
        where
            App: Appli,
            O: Path<App, Option<T>>,
            P: Path<App, T>,
            E: Element<App>,
            T: PrototypeElement<App> + 'static,
        {
            type Layouted = InnerLayouted<E::Layouted, T::Layouted>;

            fn make_layout(
                &mut self,
                state: &Context<App>,
                store: &mut ElementStore,
                generator: &mut ElementIdGenerator,
                resolver: &mut Resolver,
            ) -> Self::Layouted {
                if state.get(&self.option_path).is_some() {
                    let element = self
                        .element
                        .get_or_insert_with(|| T::to_element(self.inner_path, self.name.take().unwrap()));

                    InnerLayouted::Some(element.make_layout(state, store, generator, resolver))
                } else {
                    InnerLayouted::None(self.none_element.make_layout(state, store, generator, resolver))
                }
            }

            fn create_layout<'a>(
                &'a self,
                state: &'a Context<App>,
                store: &'a ElementStore,
                layouted: &'a Self::Layouted,
                layout: &mut Layout<'a, App>,
            ) {
                match layouted {
                    InnerLayouted::None(layouted) => self.none_element.create_layout(state, store, layouted, layout),
                    InnerLayouted::Some(layouted) => self.element.as_ref().unwrap().create_layout(state, store, layouted, layout),
                }
            }
        }

        Inner {
            name: Some(name.clone()),
            option_path: self_path,
            inner_path: self_path.unwrapped().manually_asserted(),
            none_element: split! {
                children: (
                    text! {
                        text: name,
                    },
                    text! {
                        text: "None",
                    }
                ),
            },
            element: None,
            _marker: PhantomData,
        }
    }
}

impl<App, T, const SIZE: usize> PrototypeElement<App> for [T; SIZE]
where
    App: Appli,
    T: PrototypeElement<App> + 'static,
{
    type Layouted = impl Any;
    type Return<P>
        = impl Element<App, Layouted = Self::Layouted>
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
}

// NOTE: This is generally not recommended if the type can be freely defined
// since the element store for each element is bound to the index, so changes in
// the vector might result in unexpected UI behavior. E.g. removing the first
// item of a Vec might result in the item in the second position being expanded,
// even though the now-first element should be.
// Furthermore this might also result in crashes if different instances of `T`
// require different store data (for example when using trait objects). So use
// with care!
impl<App, T> PrototypeElement<App> for Vec<T>
where
    App: Appli,
    T: PrototypeElement<App> + 'static,
{
    type Layouted = impl Any;
    type Return<P>
        = impl Element<App, Layouted = Self::Layouted>
    where
        P: Path<App, Self>;

    fn to_element<P>(self_path: P, name: String) -> Self::Return<P>
    where
        P: Path<App, Self>,
    {
        use rust_state::{ManuallyAssertExt, VecIndexExt};

        struct VecWrapper<App, T, P>
        where
            App: Appli,
            T: PrototypeElement<App>,
        {
            self_path: P,
            item_boxes: Vec<Box<dyn Element<App, Layouted = <T as PrototypeElement<App>>::Layouted>>>,
            _marker: PhantomData<T>,
        }

        impl<App, T, P> ElementSet<App> for VecWrapper<App, T, P>
        where
            App: Appli,
            T: PrototypeElement<App> + 'static,
            P: Path<App, Vec<T>>,
        {
            // TODO: Refactor to not have to re-allocate this every frame.
            type Layouted = Vec<T::Layouted>;

            fn get_element_count(&self) -> usize {
                unimplemented!("We need to take the state, store, genertor, and resolver here too to give the number of elements")
            }

            fn make_layout(
                &mut self,
                state: &Context<App>,
                store: &mut ElementStore,
                generator: &mut ElementIdGenerator,
                mut resolver_set: impl ResolverSet,
            ) -> Self::Layouted {
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
                                let item_element = PrototypeElement::to_element(item_path, index.to_string());
                                let item_box: Box<dyn Element<App, Layouted = <T as PrototypeElement<App>>::Layouted>> =
                                    Box::new(item_element);
                                item_box
                            });
                        }
                    }
                    Ordering::Equal => {}
                }

                // FIX: Make this right. Maybe with_derived should expect a resolverset as well
                resolver_set
                    .with_index(0, |resolver| {
                        resolver.with_derived(2.0, 4.0, |resolver| {
                            self.item_boxes
                                .iter_mut()
                                .enumerate()
                                .map(|(index, item_box)| {
                                    item_box.make_layout(
                                        state,
                                        store.get_or_create_child_store(index as u64, generator),
                                        generator,
                                        resolver,
                                    )
                                })
                                .collect()
                        })
                    })
                    .1
            }

            fn create_layout<'a>(
                &'a self,
                state: &'a Context<App>,
                store: &'a ElementStore,
                layouted: &'a Self::Layouted,
                layout: &mut Layout<'a, App>,
            ) {
                for (index, item_box) in self.item_boxes.iter().enumerate() {
                    item_box.create_layout(state, store.child_store(index as u64), &layouted[index], layout);
                }
            }
        }

        collapsable! {
            text: name,
            children: VecWrapper {
                self_path,
                item_boxes: Vec::new(),
                _marker: PhantomData,
            },
        }
    }
}

// impl<App, T> PrototypeElement<App> for Rc<T>
// where
//     App: Appli,
//     T: PrototypeElement<App>,
// {
//     fn to_element(&self, display: String) -> ElementCell<App> {
//         (**self).to_element(display)
//     }
// }
