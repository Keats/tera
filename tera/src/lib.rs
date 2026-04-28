//! # Tera
//!
//! A powerful, fast and easy-to-use template engine for Rust
//!
//! This crate provides an implementation of the Tera template engine, which is designed for use in
//! Rust applications. Inspired by [Jinja2] and [Django] templates, Tera provides a familiar and
//! expressive syntax for creating dynamic HTML, XML, and other text-based documents. It supports
//! template inheritance, variable interpolation, conditionals, loops, filters, and custom
//! functions, enabling developers to build complex applications with ease.
//!
//! See the [site](http://keats.github.io/tera/) for more information and to get started.
//!
//! ## Features
//!
//! - High-performance template rendering
//! - Safe and sandboxed execution environment
//! - Template inheritance and includes
//! - Expressive and familiar syntax
//! - Extensible with custom filters and functions
//! - Automatic escaping of HTML/XML by default
//! - Template caching and auto-reloading for efficient development
//! - Built-in support for JSON and other data formats
//! - Comprehensive error messages and debugging information
//!
//! ## Example
//!
//! ```rust
//! use tera::Tera;
//!
//! // Create a new Tera instance and add a template from a string
//! let mut tera = Tera::new();
//! tera.register_filter("do_nothing", do_nothing_filter);
//! tera.load_from_glob("examples/basic/templates/**/*")?;
//! // Prepare the context with some data
//! let mut context = tera::Context::new();
//! context.insert("name", "World");
//!
//! // Render the template with the given context
//! let rendered = tera.render("hello", &context)?;
//! assert_eq!(rendered, "Hello, World!");
//! ```
//!
//! ## Getting Started
//!
//! Add the following to your Cargo.toml file:
//!
//! ```toml
//! [dependencies]
//! tera = "2"
//! ```
//!
//! Then, consult the official documentation and examples to learn more about using Tera in your
//! Rust projects.
//!
//! [Jinja2]: http://jinja.pocoo.org/
//! [Django]: https://docs.djangoproject.com/en/3.1/topics/templates/

//#![deny(missing_docs)]

mod args;
mod components;
mod context;
mod delimiters;
mod errors;
mod filters;
mod functions;
#[cfg(feature = "glob_fs")]
mod globbing;
mod parsing;
mod reporting;
mod template;
mod tera;
mod tests;
mod utils;
pub mod value;
pub(crate) mod vm;

pub use crate::tera::{EscapeFn, Tera};
pub use args::Kwargs;
pub use components::{ComponentArg, ComponentArgType, ComponentInfo};
pub use context::Context;
pub use delimiters::Delimiters;
pub use errors::{Error, ErrorKind, TeraResult};
pub use filters::Filter;
pub use functions::Function;
pub use tests::Test;
pub use utils::escape_html;
pub use value::number::Number;
pub use value::{Map, Value};
pub use vm::state::State;

#[cfg(feature = "glob_fs")]
#[doc(hidden)]
pub use globbing::load_from_glob;

#[cfg(feature = "fast_hash")]
pub(crate) use ahash::{AHashMap as HashMap, AHashSet as HashSet};
#[cfg(not(feature = "fast_hash"))]
pub(crate) use std::collections::{HashMap, HashSet};

#[cfg(test)]
mod snapshot_tests;
