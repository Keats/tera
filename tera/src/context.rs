use std::borrow::Cow;
use std::collections::BTreeMap;

use serde::Serialize;

use crate::value::{Key, Value};

/// The struct that holds the context of a template rendering.
///
/// Light wrapper around a `BTreeMap` for easier insertions of Serializable
/// values
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Context {
    pub(crate) data: BTreeMap<Cow<'static, str>, Value>,
}

impl Context {
    /// Initializes an empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes something that implements Serialize and creates a context with it.
    /// Meant to be used if you have a hashmap or a struct and don't want to insert values
    /// one by one in the context.
    pub fn from_serialize<T: Serialize + ?Sized>(value: &T) -> crate::TeraResult<Self> {
        let val = Value::try_from_serializable(value)?;
        let type_name = val.name();

        match val.into_map() {
            Some(map) => {
                let mut data = BTreeMap::new();
                for (key, value) in map {
                    let key_str: Cow<'static, str> = match key {
                        Key::String(s) => Cow::Owned((*s).to_string()),
                        Key::Str(s) => Cow::Owned(s.to_string()),
                        Key::Bool(b) => Cow::Owned(b.to_string()),
                        Key::U64(u) => Cow::Owned(u.to_string()),
                        Key::I64(i) => Cow::Owned(i.to_string()),
                        Key::U128(u) => Cow::Owned(u.to_string()),
                        Key::I128(i) => Cow::Owned(i.to_string()),
                    };
                    data.insert(key_str, value);
                }
                Ok(Context { data })
            }
            None => Err(crate::Error::message(format!(
                "from_serialize requires a struct or map, got {type_name}"
            ))),
        }
    }

    /// Converts the `val` parameter to `Value` and insert it into the context.
    ///
    /// ```rust
    /// # use tera::Context;
    /// let mut context = tera::Context::new();
    /// context.insert("number_users", &42);
    /// ```
    pub fn insert<S: Into<Cow<'static, str>>, T: Serialize + ?Sized>(&mut self, key: S, val: &T) {
        self.data.insert(key.into(), Value::from_serializable(val));
    }

    /// In case you already have a `Value` you want to insert
    pub fn insert_value<S: Into<Cow<'static, str>>>(&mut self, key: S, val: Value) {
        self.data.insert(key.into(), val);
    }

    /// Remove a key from the context, returning the value at the key if the key was previously inserted into the context.
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.data.remove(key)
    }

    /// Appends the data of the `source` parameter to `self`, overwriting existing keys.
    /// The source context will be dropped.
    ///
    /// ```rust
    /// # use tera::Context;
    /// let mut target = Context::new();
    /// target.insert("a", &1);
    /// target.insert("b", &2);
    /// let mut source = Context::new();
    /// source.insert("b", &3);
    /// source.insert("d", &4);
    /// target.extend(source);
    /// ```
    pub fn extend(&mut self, mut source: Context) {
        self.data.append(&mut source.data);
    }

    /// Checks if a value exists for given key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Returns the value at the given key
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }
}

/// Creates a context from key value pairs
///
/// Example:
/// ```rust
/// let ctx = context! {
///     name => "Brian",
///     age => &24,
/// };
/// ```
/// Expands to:
/// ```
/// let ctx = {
///     let mut context = Context::new();
///     context.insert("name", "Brian");
///     context.insert("age", &24);
///     context
/// };
///
#[macro_export]
macro_rules! context {
    (
        $(
            $key:ident $(=> $value:expr)? $(,)*
        )*
    ) => {
        {
            let mut context = Context::new();
            $(
                context.insert(stringify!($key), $($value)?);
            )*
            context
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_macro_builder() {
        let left = context! {
            foo => "Bar",
            con => &69
        };

        let mut right = Context::new();
        right.insert("foo", "Bar");
        right.insert("con", &69);

        assert_eq!(left, right);
    }

    #[test]
    fn context_tests() {
        let ctx = context! {
            name => "John Doe",
            age => &42,
        };

        assert!(ctx.contains_key("age"));
        assert_eq!(ctx.get("age"), Some(&Value::from(42)));
    }

    #[test]
    fn context_from_serialize() {
        use serde::Serialize;

        #[derive(Serialize)]
        struct Person {
            name: String,
            age: i32,
        }

        let person = Person {
            name: "Alice".to_string(),
            age: 30,
        };
        let ctx = Context::from_serialize(&person).unwrap();

        assert!(ctx.contains_key("name"));
        assert!(ctx.contains_key("age"));
        assert_eq!(ctx.get("name").unwrap().as_str(), Some("Alice"));
        assert_eq!(ctx.get("age").unwrap().as_i128(), Some(30));
    }

    #[test]
    fn context_from_serialize_non_map_fails() {
        let result = Context::from_serialize(&42);
        assert!(result.is_err());
    }
}
