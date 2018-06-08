//! The immutable context

// --- module use statements ---

use serde_json::value::Value;

// --- module struct definitions ---

/// Contains the data and allows no mutation
#[derive(Debug)]
pub struct Context<'a> {
    /// Read-only context
    context_value: &'a Value,
}
/// Implementation for type `Context`.
impl<'a> Context<'a> {
    /// Create context from serde `Value`
    ///
    ///  * `context_value` - User supplied context
    ///  * _return_ - Immutable wrapper user supplied context
    ///
    #[inline]
    pub fn from_value(context_value: &'a Value) -> Context<'a> {
        Context { context_value }
    }

    /// Finds a value within the value.
    ///
    ///  * `key` - Key to find
    ///  * _return_ - Found value or `None`
    ///
    #[inline]
    pub fn find_value(self: &Self, key: &str) -> Option<&'a Value> {
        self.context_value.get(key)
    }

    /// Finds a value within the value.
    ///
    ///  * `pointer` - Key to find
    ///  * _return_ - Found value or `None`
    ///
    #[inline]
    pub fn find_value_by_pointer(self: &Self, pointer: &str) -> Option<&'a Value> {
        self.context_value.pointer(pointer)
    }

    /// Read accessor for `context_value`
    ///
    ///  * _return_ - Current state for `context_value`
    ///
    #[inline]
    pub fn context_value(& self) -> & 'a Value {
        self.context_value
    }
}

/// Test module for context module
#[cfg(test)]
mod tests {
    use super::*;
    mod context {
        use serde_json::to_value;

        #[derive(Debug, Serialize, PartialEq, Clone)]
        struct A {
            s: String,
        }

        #[derive(Debug, Serialize, PartialEq, Clone)]
        struct B {
            a: A,
        }

        #[derive(Debug, Serialize, PartialEq, Clone)]
        struct C {
            b: B,
        }

        fn sample_c() -> C {
            C {
                b: B {
                    a: A { s: "an a".into() },
                },
            }
        }

        use super::*;

        #[test]
        fn find_value() -> () {
            let c = sample_c();
            let value = to_value(&c).unwrap();

            let context = Context::from_value(&value);
            assert_eq!(context.find_value("b").unwrap(), &to_value(&c.b).unwrap());
        }

        #[test]
        fn find_value_by_pointer() -> () {
            let c = sample_c();
            let value = to_value(&c).unwrap();
            let context = Context::from_value(&value);

            assert_eq!(
                context.find_value_by_pointer("/b/a/s").unwrap(),
                &to_value("an a".to_string()).unwrap()
            );
        }
    }
}
