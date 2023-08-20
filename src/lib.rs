#![doc(html_root_url = "https://docs.rs/tera")]
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
//! - Strict mode for enforcing variable existence
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
//! let mut tera = Tera::new("templates/**/*").unwrap();
//! tera.add_raw_template("hello", "Hello, {{ name }}!").unwrap();
//! // Prepare the context with some data
//! let mut context = tera::Context::new();
//! context.insert("name", "World");
//!
//! // Render the template with the given context
//! let rendered = tera.render("hello", &context).unwrap();
//! assert_eq!(rendered, "Hello, World!");
//! ```
//!
//! ## Getting Started
//!
//! Add the following to your Cargo.toml file:
//!
//! ```toml
//! [dependencies]
//! tera = "1.0"
//! ```
//!
//! Then, consult the official documentation and examples to learn more about using Tera in your
//! Rust projects.
//!
//! [Jinja2]: http://jinja.pocoo.org/
//! [Django]: https://docs.djangoproject.com/en/3.1/topics/templates/

#![deny(missing_docs)]

#[macro_use]
mod macros;
mod builtins;
mod context;
mod errors;
mod filter_utils;
mod parser;
mod renderer;
mod template;
mod tera;
mod utils;

// Library exports.

pub use crate::builtins::filters::Filter;
pub use crate::builtins::functions::Function;
pub use crate::builtins::testers::Test;
pub use crate::context::Context;
pub use crate::errors::{Error, ErrorKind, Result};
// Template, dotted_pointer and get_json_pointer are meant to be used internally only but is exported for test/bench.
#[doc(hidden)]
pub use crate::context::dotted_pointer;
#[doc(hidden)]
#[allow(deprecated)]
pub use crate::context::get_json_pointer;
#[doc(hidden)]
pub use crate::template::Template;
pub use crate::tera::Tera;
pub use crate::utils::escape_html;
// Re-export Value and other useful things from serde
// so apps/tools can encode data in Tera types
pub use serde_json::value::{from_value, to_value, Map, Number, Value};

// Exposes the AST if one needs it but changing the AST is not considered
// a breaking change so it isn't public
#[doc(hidden)]
pub use crate::parser::ast;

/// Re-export some helper fns useful to write filters/fns/tests
pub mod helpers {
    /// Functions helping writing tests
    pub mod tests {
        pub use crate::builtins::testers::{extract_string, number_args_allowed, value_defined};
    }
}
