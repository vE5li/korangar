#![feature(adt_const_params)]
#![allow(incomplete_features)]

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
