//! Additional features for Tera that require 3rd party dependencies
//! These are in a separate package so the crate version can be changed independently
//! of the tera crate.
//!
//! To use them, call the Tera.{register_filter,register_function,register_test} functions:
//!
//! ```ignore
//! use tera::Tera;
//!
//! let mut tera = Tera::default();
//! tera.register_filter("b64_encode", tera_contrib::base64::b64_encode);
//! ```
//!
#[cfg(feature = "base64")]
pub mod base64;
#[cfg(feature = "date")]
pub mod dates;
#[cfg(feature = "filesize_format")]
pub mod filesize_format;
#[cfg(feature = "format")]
pub mod format;
#[cfg(feature = "json")]
pub mod json;
#[cfg(feature = "rand")]
pub mod rand;
#[cfg(feature = "regex")]
pub mod regex;
#[cfg(feature = "slug")]
pub mod slug;
#[cfg(feature = "urlencode")]
pub mod urlencode;
