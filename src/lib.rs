#![allow(dead_code)]

// Needed by pest
#![recursion_limit = "200"]


#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]
#![cfg_attr(feature = "dev", allow(block_in_if_condition_stmt, linkedlist))]

extern crate serde;
extern crate serde_json;
extern crate glob;
#[macro_use]
extern crate pest;
#[macro_use]
extern crate quick_error;
extern crate slug;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate url;

mod errors;
#[macro_use]
mod macros;
mod parser;
mod context;
mod render;
mod template;
mod tera;
mod filters;
mod testers;


// Library exports
// Template and Rendered are not meant to be used in your code, only there for
// bench/test of tera itself
pub use render::Renderer;
pub use template::Template;
pub use context::Context;
pub use tera::Tera;
pub use errors::{TeraResult, TeraError};
