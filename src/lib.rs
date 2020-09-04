#![doc(html_root_url = "https://docs.rs/tera")]

//! # Tera
//! Tera is a template engine based on [Jinja2](http://jinja.pocoo.org/)
//! and the [Django template language](https://docs.djangoproject.com/en/3.1/topics/templates/).
//!
//! See the [site](https://tera.netlify.com) for features and to get started.

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

// Template is meant to be used internally only but is exported for test/bench.
pub use crate::builtins::filters::Filter;
pub use crate::builtins::functions::Function;
pub use crate::builtins::testers::Test;
pub use crate::context::Context;
pub use crate::errors::{Error, ErrorKind, Result};
#[doc(hidden)]
pub use crate::template::Template;
pub use crate::tera::Tera;
pub use crate::utils::escape_html;
/// Re-export Value and other useful things from serde
/// so apps/tools can encode data in Tera types
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
