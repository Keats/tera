use serde_json::map::Map;
use serde_json::to_string_pretty;
use serde_json::value::{to_value, Number, Value};

#[derive(PartialEq, Debug)]
pub enum ForLoopKind {
    Value,
    KeyValue,
}

// We need to have some data in the renderer for when we are in a ForLoop
// For example, accessing the local variable would fail when
// looking it up in the global context
#[derive(Debug)]
pub struct ForLoop {
    /// The key name when iterate as a Key-Value, ie in `{% for i, person in people %}` it would be `i`
    pub key_name: Option<String>,
    /// The value name, ie in `{% for person in people %}` it would be `person`
    pub value_name: String,
    /// What's the current loop index (0-indexed)
    pub current: usize,
    /// A list of (key, value) for the forloop. The key is `None` for `ForLoopKind::Value`
    pub values: Vec<(Option<String>, Value)>,
    /// Value or KeyValue?
    pub kind: ForLoopKind,
    /// Values set using the {% set %} tag in forloops
    pub extra_values: Map<String, Value>,
}

impl ForLoop {
    pub fn new(value_name: &str, values: Value) -> ForLoop {
        let mut for_values = vec![];
        for val in values.as_array().unwrap() {
            for_values.push((None, val.clone()));
        }
        ForLoop {
            key_name: None,
            value_name: value_name.to_string(),
            current: 0,
            values: for_values,
            kind: ForLoopKind::Value,
            extra_values: Map::new(),
        }
    }

    pub fn new_key_value(key_name: &str, value_name: &str, values: Value) -> ForLoop {
        let mut for_values = vec![];
        for (key, val) in values.as_object().unwrap() {
            for_values.push((Some(key.clone()), val.clone()));
        }

        ForLoop {
            key_name: Some(key_name.to_string()),
            value_name: value_name.to_string(),
            current: 0,
            values: for_values,
            kind: ForLoopKind::KeyValue,
            extra_values: Map::new(),
        }
    }

    #[inline]
    pub fn increment(&mut self) {
        self.current += 1;
    }

    #[inline]
    pub fn get_current_value(&self) -> Option<&Value> {
        if let Some(v) = self.values.get(self.current) {
            return Some(&v.1);
        }
        None
    }

    /// Only called in `ForLoopKind::KeyValue`
    #[inline]
    pub fn get_current_key(&self) -> String {
        if let Some(v) = self.values.get(self.current) {
            if let Some(ref k) = v.0 {
                return k.clone();
            }
        }

        unreachable!();
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

    #[inline]
    pub fn len(&self) -> usize {
        self.values.len()
    }
}
