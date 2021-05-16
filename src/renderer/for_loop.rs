use std::borrow::Cow;

use serde_json::Value;
use unic_segment::Graphemes;

use crate::renderer::stack_frame::Val;

/// Enumerates the two types of for loops
#[derive(Debug, PartialEq)]
pub enum ForLoopKind {
    /// Loop over values, eg an `Array`
    Value,
    /// Loop over key value pairs, eg a `HashMap` or `Object` style iteration
    KeyValue,
}

/// Enumerates the states of a for loop
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ForLoopState {
    /// State during iteration
    Normal,
    /// State on encountering *break* statement
    Break,
    /// State on encountering *continue* statement
    Continue,
}

/// Enumerates on the types of values to be iterated, scalars and pairs
#[derive(Debug)]
pub enum ForLoopValues<'a> {
    /// Values for an array style iteration
    Array(Val<'a>),
    /// Values for a per-character iteration on a string
    String(Val<'a>),
    /// Values for an object style iteration
    Object(Vec<(String, Val<'a>)>),
}

impl<'a> ForLoopValues<'a> {
    pub fn current_key(&self, i: usize) -> String {
        match *self {
            ForLoopValues::Array(_) | ForLoopValues::String(_) => {
                unreachable!("No key in array list or string")
            }
            ForLoopValues::Object(ref values) => {
                values.get(i).expect("Failed getting current key").0.clone()
            }
        }
    }
    pub fn current_value(&self, i: usize) -> Val<'a> {
        match *self {
            ForLoopValues::Array(ref values) => match *values {
                Cow::Borrowed(v) => {
                    Cow::Borrowed(v.as_array().expect("Is array").get(i).expect("Value"))
                }
                Cow::Owned(_) => {
                    Cow::Owned(values.as_array().expect("Is array").get(i).expect("Value").clone())
                }
            },
            ForLoopValues::String(ref values) => {
                let mut graphemes = Graphemes::new(values.as_str().expect("Is string"));
                Cow::Owned(Value::String(graphemes.nth(i).expect("Value").to_string()))
            }
            ForLoopValues::Object(ref values) => values.get(i).expect("Value").1.clone(),
        }
    }
}

// We need to have some data in the renderer for when we are in a ForLoop
// For example, accessing the local variable would fail when
// looking it up in the global context
#[derive(Debug)]
pub struct ForLoop<'a> {
    /// The key name when iterate as a Key-Value, ie in `{% for i, person in people %}` it would be `i`
    pub key_name: Option<String>,
    /// The value name, ie in `{% for person in people %}` it would be `person`
    pub value_name: String,
    /// What's the current loop index (0-indexed)
    pub current: usize,
    /// A list of (key, value) for the forloop. The key is `None` for `ForLoopKind::Value`
    pub values: ForLoopValues<'a>,
    /// Value or KeyValue?
    pub kind: ForLoopKind,
    /// Has the for loop encountered break or continue?
    pub state: ForLoopState,
}

impl<'a> ForLoop<'a> {
    pub fn from_array(value_name: &str, values: Val<'a>) -> Self {
        ForLoop {
            key_name: None,
            value_name: value_name.to_string(),
            current: 0,
            values: ForLoopValues::Array(values),
            kind: ForLoopKind::Value,
            state: ForLoopState::Normal,
        }
    }

    pub fn from_string(value_name: &str, values: Val<'a>) -> Self {
        ForLoop {
            key_name: None,
            value_name: value_name.to_string(),
            current: 0,
            values: ForLoopValues::String(values),
            kind: ForLoopKind::Value,
            state: ForLoopState::Normal,
        }
    }

    pub fn from_object(key_name: &str, value_name: &str, object: &'a Value) -> Self {
        let object_values = object.as_object().unwrap();
        let mut values = Vec::with_capacity(object_values.len());
        for (k, v) in object_values {
            values.push((k.to_string(), Cow::Borrowed(v)));
        }

        ForLoop {
            key_name: Some(key_name.to_string()),
            value_name: value_name.to_string(),
            current: 0,
            values: ForLoopValues::Object(values),
            kind: ForLoopKind::KeyValue,
            state: ForLoopState::Normal,
        }
    }

    pub fn from_object_owned(key_name: &str, value_name: &str, object: Value) -> Self {
        let object_values = match object {
            Value::Object(c) => c,
            _ => unreachable!(
                "Tried to create a Forloop from an object owned but it wasn't an object"
            ),
        };
        let mut values = Vec::with_capacity(object_values.len());
        for (k, v) in object_values {
            values.push((k.to_string(), Cow::Owned(v)));
        }

        ForLoop {
            key_name: Some(key_name.to_string()),
            value_name: value_name.to_string(),
            current: 0,
            values: ForLoopValues::Object(values),
            kind: ForLoopKind::KeyValue,
            state: ForLoopState::Normal,
        }
    }

    #[inline]
    pub fn increment(&mut self) {
        self.current += 1;
        self.state = ForLoopState::Normal;
    }

    pub fn is_key_value(&self) -> bool {
        self.kind == ForLoopKind::KeyValue
    }

    #[inline]
    pub fn break_loop(&mut self) {
        self.state = ForLoopState::Break;
    }

    #[inline]
    pub fn continue_loop(&mut self) {
        self.state = ForLoopState::Continue;
    }

    #[inline]
    pub fn get_current_value(&self) -> Val<'a> {
        self.values.current_value(self.current)
    }

    /// Only called in `ForLoopKind::KeyValue`
    #[inline]
    pub fn get_current_key(&self) -> String {
        self.values.current_key(self.current)
    }

    /// Checks whether the key string given is the variable used as key for
    /// the current forloop
    pub fn is_key(&self, name: &str) -> bool {
        if self.kind == ForLoopKind::Value {
            return false;
        }

        if let Some(ref key_name) = self.key_name {
            return key_name == name;
        }

        false
    }

    pub fn len(&self) -> usize {
        match self.values {
            ForLoopValues::Array(ref values) => values.as_array().expect("Value is array").len(),
            ForLoopValues::String(ref values) => {
                values.as_str().expect("Value is string").chars().count()
            }
            ForLoopValues::Object(ref values) => values.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use serde_json::Value;

    use super::ForLoop;

    #[test]
    fn test_that_iterating_on_string_yields_grapheme_clusters() {
        let text = "a\u{310}e\u{301}o\u{308}\u{332}".to_string();
        let string = Value::String(text.clone());
        let mut string_loop = ForLoop::from_string("whatever", Cow::Borrowed(&string));
        assert_eq!(*string_loop.get_current_value(), text[0..3]);
        string_loop.increment();
        assert_eq!(*string_loop.get_current_value(), text[3..6]);
        string_loop.increment();
        assert_eq!(*string_loop.get_current_value(), text[6..]);
    }
}
