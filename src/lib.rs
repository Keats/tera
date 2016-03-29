#![allow(dead_code)]

#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

extern crate serde;
extern crate serde_json;
extern crate glob;

mod lexer;
mod nodes;
mod parser;
mod context;
mod render;
mod template;
mod tera;


// Library exports
// Template is not meant to be used in your code, only there for bench/test of
// tera itself
pub use template::Template;
pub use context::Context;
pub use tera::Tera;
