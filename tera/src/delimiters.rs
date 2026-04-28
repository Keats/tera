use std::borrow::Cow;

use crate::errors::{Error, TeraResult};

/// This allows customizing the delimiters used for blocks, variables, and comments in case
/// you want to template files that contains text like `{{`, like LaTeX.
/// Delimiters need to be 2 ASCII characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delimiters {
    /// Start delimiter for blocks, default: `{%`
    pub block_start: Cow<'static, str>,
    /// End delimiter for blocks, default: `%}`
    pub block_end: Cow<'static, str>,
    /// Start delimiter for variables, default: `{{`
    pub variable_start: Cow<'static, str>,
    /// End delimiter for variables, default: `}}`
    pub variable_end: Cow<'static, str>,
    /// Start delimiter for comments, default: `{#`
    pub comment_start: Cow<'static, str>,
    /// End delimiter for comments, default: `#}`
    pub comment_end: Cow<'static, str>,
}

impl Default for Delimiters {
    fn default() -> Self {
        Self {
            block_start: "{%".into(),
            block_end: "%}".into(),
            variable_start: "{{".into(),
            variable_end: "}}".into(),
            comment_start: "{#".into(),
            comment_end: "#}".into(),
        }
    }
}

impl Delimiters {
    /// Returns an error if any delimiter is empty or if there are conflicts
    pub(crate) fn validate(&self) -> TeraResult<()> {
        if self.block_start.chars().count() != 2 {
            return Err(Error::message(
                "`block_start` delimiter must be 2 characters",
            ));
        }
        if self.block_end.chars().count() != 2 {
            return Err(Error::message("`block_end` delimiter must be 2 characters"));
        }
        if self.variable_start.chars().count() != 2 {
            return Err(Error::message(
                "`variable_start` delimiter must be 2 characters",
            ));
        }
        if self.variable_end.chars().count() != 2 {
            return Err(Error::message(
                "`variable_end` delimiter must be 2 characters",
            ));
        }
        if self.comment_start.chars().count() != 2 {
            return Err(Error::message(
                "`comment_start` delimiter must be 2 characters",
            ));
        }
        if self.comment_end.chars().count() != 2 {
            return Err(Error::message(
                "`comment_end` delimiter must be 2 characters",
            ));
        }

        // Check for conflicting start delimiters
        if self.block_start == self.variable_start {
            return Err(Error::message(
                "`block_start` and `variable_start` cannot have the same value",
            ));
        }
        if self.block_start == self.comment_start {
            return Err(Error::message(
                "`block_start` and `comment_start` cannot have the same value",
            ));
        }
        if self.variable_start == self.comment_start {
            return Err(Error::message(
                "`variable_start` and `comment_start` cannot have the same value",
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_on_invalid_delimiters() {
        let inputs = vec![
            Delimiters {
                block_start: "".into(),
                ..Delimiters::default()
            },
            Delimiters {
                block_start: "[[[".into(),
                ..Delimiters::default()
            },
            Delimiters {
                block_start: "[[".into(),
                comment_start: "[[".into(),
                ..Delimiters::default()
            },
        ];

        for i in inputs {
            assert!(i.validate().is_err());
        }
    }
}
