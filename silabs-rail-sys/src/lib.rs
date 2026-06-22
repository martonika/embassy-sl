#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(clippy::all)]

//! Low-level FFI bindings to Silicon Labs RAIL.
//!
//! This crate links the proprietary RAIL static library from the Simplicity SDK
//! (MSLA). Set `SILABS_SDK` to your local SDK path before building.
//!
//! Prefer the safe wrappers in `embassy-silabs` with the `rail` feature.

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
