#![doc(html_root_url = "https://docs.rs/tera/0.11")]

//! # Tera
//! Tera is a template engine based on [Jinja2](http://jinja.pocoo.org/)
//! and the [Django template language](https://docs.djangoproject.com/en/1.9/topics/templates/).
//!
//! See the [site](https://tera.netlify.com) for features and to get started.

#![deny(missing_docs)]

extern crate glob;
extern crate pest;
extern crate serde;
#[cfg_attr(test, macro_use)]
extern crate serde_json;
#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate error_chain;
extern crate regex;
extern crate slug;
#[macro_use]
extern crate lazy_static;
extern crate chrono;
extern crate humansize;
extern crate url;
#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;
#[cfg(test)]
#[macro_use]
extern crate serde_derive;
extern crate unic_segment;

#[macro_use]
mod macros;
mod builtins;
mod context;
mod errors;
mod parser;
mod renderer;
mod sort_utils;
mod template;
mod tera;
mod utils;

// Library exports.

// Template is meant to be used internally only but is exported for test/bench.
pub use builtins::filters::FilterFn;
pub use builtins::functions::GlobalFn;
pub use builtins::testers::TesterFn;
pub use context::Context;
pub use errors::{Error, ErrorKind, Result};
/// Re-export Value and other useful things from serde
/// so apps/tools can encode data in Tera types
pub use serde_json::value::{from_value, to_value, Map, Number, Value};
#[doc(hidden)]
pub use template::Template;
pub use tera::Tera;
pub use utils::escape_html;

// Exposes the AST if one needs it but changing the AST is not considered
// a breaking change so it isn't public
#[doc(hidden)]
pub use parser::ast;
