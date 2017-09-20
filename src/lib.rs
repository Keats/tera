//! # Tera
//! Tera is a template engine based on [Jinja2](http://jinja.pocoo.org/)
//! and the [Django template language](https://docs.djangoproject.com/en/1.9/topics/templates/).
//!
//! See the [README](https://github.com/Keats/tera/blob/master/README.md) for features and
//! to get started.

#![allow(missing_docs)]
//#![deny(missing_docs)]
#![allow(unused)]

#![cfg_attr(feature = "cargo-clippy", allow(block_in_if_condition_stmt, eq_op))]

extern crate serde;
extern crate serde_json;
extern crate glob;
extern crate pest;
#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate error_chain;
extern crate slug;
extern crate regex;
#[macro_use]
extern crate lazy_static;
extern crate url;
extern crate humansize;
extern crate chrono;
#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

#[macro_use]
mod macros;
mod errors;
mod context;
mod parser;
mod template;
mod utils;
mod builtins;
mod renderer;
mod tera;


// Library exports.

// Template is meant to be used internally only but is exported for test/bench.
#[doc(hidden)] pub use template::Template;
pub use context::Context;
pub use tera::Tera;
pub use errors::{Result, Error, ErrorKind};
pub use utils::{escape_html};
pub use builtins::global_functions::GlobalFn;
pub use builtins::filters::FilterFn;
pub use builtins::testers::TesterFn;
//// Re-export Value so apps/tools can encode data in Tera types
//// for now it's just an alias to serde_json::Value
pub use serde_json::value::{Value, from_value, to_value};

// Exposes the AST if one needs it but changing the AST is not considered
// a breaking change so it isn't public
#[doc(hidden)]
pub use parser::ast;
