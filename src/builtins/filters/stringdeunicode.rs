use crate::errors::{Error, Result};
use crate::filter_utils::{GetValue, SortPairs};
use deunicode;
use serde_json::Value;
use std::ops::Deref;

#[derive(Clone, PartialOrd, Ord, Eq, PartialEq, Default)]
pub struct StringDeunicode(String);

impl Deref for StringDeunicode {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type SortStringsDeunicode = SortPairs<StringDeunicode>;

impl GetValue for StringDeunicode {
    fn get_value(val: &Value) -> Result<Self> {
        let str: Result<&str> =
            val.as_str().ok_or_else(|| Error::msg(format!("expected string got {}", val)));
        Ok(StringDeunicode(deunicode::deunicode(str?)))
    }
}
