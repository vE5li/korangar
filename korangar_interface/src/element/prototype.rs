use std::array;
use std::cell::UnsafeCell;
use std::cmp::Ordering;
use std::fmt::Display;
use std::marker::PhantomData;
use std::rc::Rc;

use interface_macros::{button, collapsable};
use rust_state::{ArrayLookupExt, Context, ManuallyAssertExt, Path};

use super::Element;
use crate::application::Appli;
use crate::element::ElementSet;
use crate::element::id::ElementIdGenerator;
use crate::element::store::ElementStore;
use crate::event::EventQueue;
use crate::layout::{Layout, Resolver};

// TODO: Rename this to StateElement
pub trait PrototypeElement<App: Appli> {
    fn to_element(self_path: impl Path<App, Self>, name: String) -> impl Element<App>;

    // TODO: Add `to_element_mut`
}

pub trait ElementDisplay {
    fn display(&self) -> String;
}

// workaround for not having negative trait bounds or better specialization
auto trait NoDisplay {}
impl !NoDisplay for f32 {}
#[cfg(feature = "cgmath")]
impl<T> !NoDisplay for cgmath::Point3<T> {}
#[cfg(feature = "cgmath")]
impl<T> !NoDisplay for cgmath::Vector2<T> {}
#[cfg(feature = "cgmath")]
impl<T> !NoDisplay for cgmath::Vector3<T> {}
#[cfg(feature = "cgmath")]
impl<T> !NoDisplay for cgmath::Vector4<T> {}
#[cfg(feature = "cgmath")]
impl<T> !NoDisplay for cgmath::Quaternion<T> {}
#[cfg(feature = "cgmath")]
impl<T> !NoDisplay for cgmath::Rad<T> {}

impl<T> ElementDisplay for T
where
    T: Display + NoDisplay,
{
    fn display(&self) -> String {
        self.to_string()
    }
}

impl ElementDisplay for f32 {
    fn display(&self) -> String {
        format!("{self:.1}")
    }
}

#[cfg(feature = "cgmath")]
impl<T: ElementDisplay> ElementDisplay for cgmath::Point3<T> {
    fn display(&self) -> String {
        format!("{}, {}, {}", self.x.display(), self.y.display(), self.z.display())
    }
}

#[cfg(feature = "cgmath")]
impl<T: ElementDisplay> ElementDisplay for cgmath::Vector2<T> {
    fn display(&self) -> String {
        format!("{}, {}", self.x.display(), self.y.display())
    }
}

#[cfg(feature = "cgmath")]
impl<T: ElementDisplay> ElementDisplay for cgmath::Vector3<T> {
    fn display(&self) -> String {
        format!("{}, {}, {}", self.x.display(), self.y.display(), self.z.display())
    }
}

#[cfg(feature = "cgmath")]
impl<T: ElementDisplay> ElementDisplay for cgmath::Vector4<T> {
    fn display(&self) -> String {
        format!(
            "{}, {}, {}, {}",
            self.x.display(),
            self.y.display(),
            self.z.display(),
            self.w.display()
        )
    }
}

#[cfg(feature = "cgmath")]
impl<T: ElementDisplay> ElementDisplay for cgmath::Quaternion<T> {
    fn display(&self) -> String {
        format!(
            "{:.1}, {:.1}, {:.1} - {:.1}",
            self.v.x.display(),
            self.v.y.display(),
            self.v.z.display(),
            self.s.display()
        )
    }
}

#[cfg(feature = "cgmath")]
impl<T: ElementDisplay> ElementDisplay for cgmath::Rad<T> {
    fn display(&self) -> String {
        self.0.display()
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
    fn to_element(self_path: impl Path<App, Self>, name: String) -> impl Element<App> {
        button! {
            text: name,
            event: move |state: &Context<App>, _: &mut EventQueue<App>| {
                let value = state.get(&self_path);
                let display = value.display();

                println!("Value is {}", display);
            },
        }
    }
}

impl<App, T> PrototypeElement<App> for std::sync::Arc<T>
where
    App: Appli,
    T: PrototypeElement<App>,
{
    fn to_element(self_path: impl Path<App, Self>, name: String) -> impl Element<App> {
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
    T: PrototypeElement<App>,
{
    fn to_element(self_path: impl Path<App, Self>, name: String) -> impl Element<App> {
        use korangar_interface::prelude::*;

        button! {
            text: name,
            event: |_: &rust_state::Context<App>, _: &mut EventQueue<App>| {
                println!("Just a dummy for now");
            },
        }
    }
}

impl<App, T, const SIZE: usize> PrototypeElement<App> for [T; SIZE]
where
    App: Appli,
    T: PrototypeElement<App> + 'static,
{
    fn to_element(self_path: impl Path<App, Self>, name: String) -> impl Element<App> {
        let elements: [impl Element<App>; SIZE] = array::from_fn(|index| {
            let item_path = self_path.array_index(index).manually_asserted();
            T::to_element(item_path, index.to_string())
        });

        collapsable! { text: name, children: elements }
    }
}

// impl<App, T> PrototypeElement<App> for Vec<T>
// where
//     App: Appli,
//     T: VecItem + PrototypeElement<App> + 'static,
//     T::Id: Display + Ord + TryInto<usize>,
// {
//     fn to_element(self_path: impl Path<App, Self>, name: String) -> impl
// Element<App> {         use rust_state::{ManuallyAssertExt, VecLookupExt};
//
//         struct VecWrapper<App, T, P>
//         where
//             App: Appli,
//             T: VecItem,
//             T::Id: Ord,
//         {
//             self_path: P,
//             item_boxes: UnsafeCell<BTreeMap<T::Id, Box<dyn Element<App>>>>,
//             _marker: PhantomData<T>,
//         }
//
//         impl<App, T, P> ElementSet<App> for VecWrapper<App, T, P>
//         where
//             App: Appli,
//             T: VecItem + PrototypeElement<App> + 'static,
//             T::Id: Display + Ord + TryInto<usize>,
//             P: Path<App, Vec<T>>,
//         {
//             fn get_height(&self, state: &Context<App>, store: &ElementStore,
// generator: &mut ElementIdGenerator, resolver: &mut Resolver) {
// let vector = state.get(&self.self_path);                 let item_boxes =
// unsafe { &mut *self.item_boxes.get() };
//
//                 // Delete old items.
//                 // TODO: Optimize this.
//                 item_boxes.retain(|key, _| vector.iter().any(|item|
// item.get_id() == *key));
//
//                 // Add new items
//                 for item in vector {
//                     let id = item.get_id();
//
//                     item_boxes.entry(id).or_insert_with(|| {
//                         let item_path =
// self.self_path.lookup(id).manually_asserted();                         let
// item_element = PrototypeElement::to_element(item_path, id.to_string());
//                         let item_box: Box<dyn Element<App>> =
// Box::new(item_element);                         item_box
//                     });
//                 }
//
//                 resolver.with_derived(2.0, 4.0, |resolver| {
//                     for (id, item_box) in item_boxes {
//                         // TODO: Cleanup
//                         let id: usize = (*id).try_into().map_err(|_| "failed
// to get id").unwrap();                         item_box.get_height(state,
// store.child_store(id as u64, generator), generator, resolver);
// }                 });
//             }
//
//             fn create_layout<'a>(
//                 &'a self,
//                 state: &'a Context<App>,
//                 store: &'a ElementStore,
//                 generator: &mut ElementIdGenerator,
//                 resolver: &mut Resolver,
//                 layout: &mut Layout<'a, App>,
//             ) {
//                 let vector = state.get(&self.self_path);
//                 let item_boxes = unsafe { &mut *self.item_boxes.get() };
//
//                 // Delete old items.
//                 // TODO: Optimize this.
//                 item_boxes.retain(|key, _| vector.iter().any(|item|
// item.get_id() == *key));
//
//                 // Add new items
//                 for item in vector {
//                     let id = item.get_id();
//
//                     item_boxes.entry(id).or_insert_with(|| {
//                         let item_path =
// self.self_path.lookup(id).manually_asserted();                         let
// item_element = PrototypeElement::to_element(item_path, id.to_string());
//                         let item_box: Box<dyn Element<App>> =
// Box::new(item_element);                         item_box
//                     });
//                 }
//
//                 resolver.with_derived(2.0, 4.0, |resolver| {
//                     // TODO: Very much temp
//                     layout.push_layer();
//
//                     for (id, item_box) in item_boxes {
//                         // TODO: Cleanup
//                         let id: usize = (*id).try_into().map_err(|_| "failed
// to get id").unwrap();                         item_box.create_layout(state,
// store.child_store(id as u64, generator), generator, resolver, layout);
//                     }
//
//                     // TODO: Very much temp
//                     layout.pop_layer();
//                 });
//             }
//         }
//
//         collapsable! {
//             text: name,
//             children: VecWrapper {
//                 self_path,
//                 item_boxes: UnsafeCell::new(BTreeMap::new()),
//                 _marker: PhantomData,
//             },
//         }
//     }
// }

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
    fn to_element(self_path: impl Path<App, Self>, name: String) -> impl Element<App> {
        use rust_state::{ManuallyAssertExt, VecIndexExt};

        struct VecWrapper<App, T, P>
        where
            App: Appli,
        {
            self_path: P,
            item_boxes: UnsafeCell<Vec<Box<dyn Element<App>>>>,
            _marker: PhantomData<T>,
        }

        impl<App, T, P> ElementSet<App> for VecWrapper<App, T, P>
        where
            App: Appli,
            T: PrototypeElement<App> + 'static,
            P: Path<App, Vec<T>>,
        {
            fn get_height(&self, state: &Context<App>, store: &ElementStore, generator: &mut ElementIdGenerator, resolver: &mut Resolver) {
                let vector = state.get(&self.self_path);
                let item_boxes = unsafe { &mut *self.item_boxes.get() };

                match item_boxes.len().cmp(&vector.len()) {
                    Ordering::Greater => {
                        // Delete excess elements.
                        item_boxes.truncate(vector.len());
                    }
                    Ordering::Less => {
                        // Add new elements.
                        for index in item_boxes.len()..vector.len() {
                            item_boxes.push({
                                let item_path = self.self_path.index(index).manually_asserted();
                                let item_element = PrototypeElement::to_element(item_path, index.to_string());
                                let item_box: Box<dyn Element<App>> = Box::new(item_element);
                                item_box
                            });
                        }
                    }
                    Ordering::Equal => {}
                }

                resolver.with_derived(2.0, 4.0, |resolver| {
                    for (index, item_box) in item_boxes.iter().enumerate() {
                        item_box.get_height(state, store.child_store(index as u64, generator), generator, resolver);
                    }
                });
            }

            fn create_layout<'a>(
                &'a self,
                state: &'a Context<App>,
                store: &'a ElementStore,
                generator: &mut ElementIdGenerator,
                resolver: &mut Resolver,
                layout: &mut Layout<'a, App>,
            ) {
                let vector = state.get(&self.self_path);
                let item_boxes = unsafe { &mut *self.item_boxes.get() };

                match item_boxes.len().cmp(&vector.len()) {
                    Ordering::Greater => {
                        // Delete excess elements.
                        item_boxes.truncate(vector.len());
                    }
                    Ordering::Less => {
                        // Add new elements.
                        for index in item_boxes.len()..vector.len() {
                            item_boxes.push({
                                let item_path = self.self_path.index(index).manually_asserted();
                                let item_element = PrototypeElement::to_element(item_path, index.to_string());
                                let item_box: Box<dyn Element<App>> = Box::new(item_element);
                                item_box
                            });
                        }
                    }
                    Ordering::Equal => {}
                }

                resolver.with_derived(2.0, 4.0, |resolver| {
                    // TODO: Very much temp
                    layout.push_layer();

                    for (index, item_box) in item_boxes.iter().enumerate() {
                        item_box.create_layout(state, store.child_store(index as u64, generator), generator, resolver, layout);
                    }

                    // TODO: Very much temp
                    layout.pop_layer();
                });
            }
        }

        collapsable! {
            text: name,
            children: VecWrapper {
                self_path,
                item_boxes: UnsafeCell::new(Vec::new()),
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
