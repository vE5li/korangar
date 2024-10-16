#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![feature(unsized_const_params)]
#![cfg_attr(feature = "interface", feature(negative_impls))]
#![cfg_attr(feature = "interface", feature(impl_trait_in_assoc_type))]

pub mod action;
pub mod archive;
pub mod color;
pub mod effect;
pub mod map;
pub mod model;
pub mod signature;
pub mod sprite;
pub mod transform;
pub mod version;

// To make proc macros work in ragnarok_formats.
extern crate self as ragnarok_formats;
