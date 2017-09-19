//! # Tera
//! Tera is a template engine based on [Jinja2](http://jinja.pocoo.org/)
//! and the [Django template language](https://docs.djangoproject.com/en/1.9/topics/templates/).
//!
//! See the [README](https://github.com/Keats/tera/blob/master/README.md) for features and
//! to get started.

// Needed by pest
#![recursion_limit = "300"]
#![deny(missing_docs)]
#![allow(unused)]

#![cfg_attr(feature = "cargo-clippy", allow(block_in_if_condition_stmt, linkedlist, eq_op))]

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

mod errors;
//mod context;
#[macro_use]
mod macros;
mod parser;
//mod template;

//mod parser_old;
//mod render;
//mod tera;
//mod filters;
//mod testers;
//mod utils;
//mod global_functions;


// Library exports.

// Template is meant to be used internally only but is exported for test/bench.
//#[doc(hidden)] pub use template::Template;
//pub use context::Context;
//pub use tera::Tera;
//pub use errors::{Result, Error, ErrorKind};
//pub use utils::{escape_html};
//pub use global_functions::GlobalFn;
//pub use filters::FilterFn;
//pub use testers::TesterFn;
//// Re-export Value so apps/tools can encode data in Tera types
//// for now it's serde_json
//pub use serde_json::value::{Value, from_value, to_value};

// Exposes the AST if one needs it but changing the AST is not considered
// a breaking change so it isn't public
//#[doc(hidden)]
//pub mod ast {
//    pub use parser_old::Operator;
//    pub use parser_old::Node;
//}
