#![warn(future_incompatible)]
#![warn(rust_2021_compatibility)]
#![warn(missing_debug_implementations)]
#![forbid(overflowing_literals)]

pub mod byte_code;
pub mod machine;
pub mod machine_io;
pub mod source_file;
mod utility;

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDocTests;
