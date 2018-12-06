use serde_json;
use std::convert::Into;
use std::error::Error as StdError;
use std::fmt;

/// The kind of an error (non-exhaustive)
#[derive(Debug)]
pub enum ErrorKind {
    /// Generic error
    Msg(String),
    /// A loop was found while looking up the inheritance chain
    CircularExtend {
        /// Name of the template with the loop
        tpl: String,
        /// All the parents templates we found so far
        inheritance_chain: Vec<String>,
    },
    /// A template is extending a template that wasn't found in the Tera instance
    MissingParent {
        /// The template we are currently looking at
        current: String,
        /// The missing template
        parent: String,
    },
    /// A template was missing (more generic version of MissingParent)
    TemplateNotFound(String),
    /// A filter wasn't found
    FilterNotFound(String),
    /// A test wasn't found
    TestNotFound(String),
    /// A function wasn't found
    FunctionNotFound(String),
    /// An error happened while serializing JSON
    Json(serde_json::Error),
    /// This enum may grow additional variants, so this makes sure clients
    /// don't count on exhaustive matching. (Otherwise, adding a new variant
    /// could break existing code.)
    #[doc(hidden)]
    __Nonexhaustive,
}

/// The Error type
#[derive(Debug)]
pub struct Error {
    /// Kind of error
    pub kind: ErrorKind,
    cause: Option<Box<dyn StdError>>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            ErrorKind::Msg(ref message) => write!(f, "{}", message),
            ErrorKind::CircularExtend { ref tpl, ref inheritance_chain } => write!(
                f,
                "Circular extend detected for template '{}'. Inheritance chain: `{:?}`",
                tpl, inheritance_chain
            ),
            ErrorKind::MissingParent { ref current, ref parent } => write!(
                f,
                "Template '{}' is inheriting from '{}', which doesn't exist or isn't loaded.",
                current, parent
            ),
            ErrorKind::TemplateNotFound(ref name) => write!(f, "Template '{}' not found", name),
            ErrorKind::FilterNotFound(ref name) => write!(f, "Filter '{}' not found", name),
            ErrorKind::TestNotFound(ref name) => write!(f, "Test '{}' not found", name),
            ErrorKind::FunctionNotFound(ref name) => write!(f, "Function '{}' not found", name),
            ErrorKind::Json(ref e) => write!(f, "{}", e),
            ErrorKind::__Nonexhaustive => write!(f, "Nonexhaustive"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.cause.as_ref().map(|c| &**c)
    }
}

impl Error {
    /// Creates generic error
    pub fn msg(value: impl ToString) -> Self {
        Self { kind: ErrorKind::Msg(value.to_string()), cause: None }
    }

    /// Creates a circular extend error
    pub fn circular_extend(tpl: impl ToString, inheritance_chain: Vec<String>) -> Self {
        Self {
            kind: ErrorKind::CircularExtend { tpl: tpl.to_string(), inheritance_chain },
            cause: None,
        }
    }

    /// Creates a missing parent error
    pub fn missing_parent(current: impl ToString, parent: impl ToString) -> Self {
        Self {
            kind: ErrorKind::MissingParent {
                current: current.to_string(),
                parent: parent.to_string(),
            },
            cause: None,
        }
    }

    /// Creates a template not found error
    pub fn template_not_found(tpl: impl ToString) -> Self {
        Self { kind: ErrorKind::TemplateNotFound(tpl.to_string()), cause: None }
    }

    /// Creates a filter not found error
    pub fn filter_not_found(name: impl ToString) -> Self {
        Self { kind: ErrorKind::FilterNotFound(name.to_string()), cause: None }
    }

    /// Creates a test not found error
    pub fn test_not_found(name: impl ToString) -> Self {
        Self { kind: ErrorKind::TestNotFound(name.to_string()), cause: None }
    }

    /// Creates a function not found error
    pub fn function_not_found(name: impl ToString) -> Self {
        Self { kind: ErrorKind::FunctionNotFound(name.to_string()), cause: None }
    }

    /// Creates generic error with a cause
    pub fn chain(value: impl ToString, cause: impl Into<Box<dyn StdError>>) -> Self {
        Self { kind: ErrorKind::Msg(value.to_string()), cause: Some(cause.into()) }
    }

    /// Creates JSON error
    pub fn json(value: serde_json::Error) -> Self {
        Self { kind: ErrorKind::Json(value), cause: None }
    }

    /// Iterate on all the error sources
    pub fn iter(&self) -> Iter {
        Iter::new(Some(self))
    }
}

#[derive(Debug)]
pub struct Iter<'a>(Option<&'a dyn StdError>);

impl<'a> Iter<'a> {
    pub fn new(err: Option<&'a dyn StdError>) -> Iter<'a> {
        Iter(err)
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a dyn StdError;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.take() {
            Some(e) => {
                self.0 = e.cause();
                Some(e)
            }
            None => None,
        }
    }
}

/// Convenient wrapper around std::Result.
pub type Result<T> = ::std::result::Result<T, Error>;
