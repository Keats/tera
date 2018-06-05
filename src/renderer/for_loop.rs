//! Provides implementation for rendering for loops

// --- module use statements ---

use renderer::ref_or_owned::RefOrOwned;
use serde_json::{to_value, Value};

// --- module enum definitions ---

/// Enumerates the two types of for loops
#[derive(Debug, PartialEq)]
enum ForLoopKind {
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

/// Enumerates on the two types of values to be iterated, scalars and pairs
#[derive(Debug)]
pub enum Values<'a> {
  /// Values for an array style iteration
  ArrayValues {
    /// Array style values
    values: RefOrOwned<'a, Value>,
  },
  /// Values for an object style iteration
  ObjectValues {
    /// Values for Object style iteration
    values: Vec<(RefOrOwned<'a, Value>, RefOrOwned<'a, Value>)>,
  },
}

// --- module struct definitions ---

/// Provides support for rendering of for loops by tracking loop state
#[derive(Debug)]
pub struct ForLoop<'a> {
  /// The key name when iterate as a Key-Value, ie in `{% for i, person in people %}` it would be `i`
  key_name: Option<&'a str>,
  ///  The value name, ie in `{% for person in people %}` it would be `person`
  value_name: &'a str,
  /// What's the current loop index (0-indexed)
  current: usize,
  /// Kind of for loop
  for_loop_kind: ForLoopKind,
  /// Values to iterate on
  values: Values<'a>,
  /// Current state of loop
  for_loop_state: ForLoopState,
}
/// Implementation for type `ForLoop`.
impl<'a> ForLoop<'a> {
  /// Crate an array style `ForLoop`
  ///
  ///  * `value_name` - The value name in the iteration
  ///  * `array` - The object to iterate on
  ///  * _return_ - Created `ForLoop`
  ///
  #[inline]
  pub fn from_array(value_name: &'a str, array: RefOrOwned<'a, Value>) -> ForLoop<'a> {
    ForLoop {
      key_name: None,
      value_name: value_name,
      current: 0,
      values: Values::ArrayValues { values: array },
      for_loop_kind: ForLoopKind::Value,
      for_loop_state: ForLoopState::Normal,
    }
  }

  /// Crate an array style `ForLoop`
  ///
  ///  * `key_name` - The key˝ name in the iteration
  ///  * `value_name` - The value name in the iteration
  ///  * `object` - The object with values to iterate on
  ///  * _return_ - Created `ForLoop`
  ///
  pub fn from_object(key_name: &'a str, value_name: &'a str, object: &'a Value) -> ForLoop<'a> {
    let object_values = object.as_object().unwrap();
    let mut values = Vec::with_capacity(object_values.len());
    for (k, v) in object_values {
      println!("PUSHING KEY {}, VAL {:?}", k, v);
      values.push((
        RefOrOwned::from_owned(to_value(k.clone()).expect("String to Value")),
        RefOrOwned::from_borrow(v),
      ));
    }

    ForLoop {
      key_name: Some(key_name),
      value_name: value_name,
      current: 0,
      values: Values::ObjectValues { values },
      for_loop_kind: ForLoopKind::KeyValue,
      for_loop_state: ForLoopState::Normal,
    }
  }

  /// Increment the for loop
  ///
  #[inline]
  pub fn increment(&mut self) -> () {
    self.current += 1;
    self.for_loop_state = ForLoopState::Normal;
  }

  /// Set state of loop to break
  ///
  #[inline]
  pub fn break_loop(&mut self) -> () {
    self.for_loop_state = ForLoopState::Break;
  }

  /// Set state of loop to continue
  ///
  #[inline]
  pub fn continue_loop(&mut self) -> () {
    self.for_loop_state = ForLoopState::Continue;
  }

  /// Get key of Object style loop for current iteration
  ///
  ///  * _return_ - Key for current iteration
  ///
  #[inline]
  pub fn current_key(&self) -> RefOrOwned<'a, Value> {
    // custom <fn for_loop_current_key>

    self.values.current_key(self.current)

    // end <fn for_loop_current_key>
  }

  /// Get value of Object style loop for current iteration
  ///
  ///  * _return_ - Value for current iteration
  ///
  #[inline]
  pub fn current_value(&self) -> RefOrOwned<'a, Value> {
    // custom <fn for_loop_current_value>

    self.values.current_value(self.current)

    // end <fn for_loop_current_value>
  }

  /// Returns number of values in for loop
  ///
  ///  * _return_ - Number of values
  ///
  #[inline]
  pub fn len(&self) -> usize {
    match &self.values {
      Values::ArrayValues { values } => values.as_array().expect("Value is array").len(),
      Values::ObjectValues { values } => values.len(),
    }
  }

  /// Read accessor for `key_name`
  ///
  ///  * _return_ - Current state for `key_name`
  ///
  #[inline]
  pub fn key_name(&self) -> Option<&'a str> {
    self.key_name
  }

  /// Read accessor for `value_name`
  ///
  ///  * _return_ - Current state for `value_name`
  ///
  #[inline]
  pub fn value_name(&self) -> &'a str {
    self.value_name
  }

  /// Read accessor for `current`
  ///
  ///  * _return_ - Current state for `current`
  ///
  #[inline]
  pub fn current(&self) -> usize {
    self.current
  }

  /// Read accessor for `values`
  ///
  ///  * _return_ - Current state for `values`
  ///
  #[inline]
  pub fn values(&self) -> &Values<'a> {
    &self.values
  }

  /// Read accessor for `for_loop_state`
  ///
  ///  * _return_ - Current state for `for_loop_state`
  ///
  #[inline]
  pub fn for_loop_state(&self) -> ForLoopState {
    self.for_loop_state
  }

  // custom <impl for_loop>
  // end <impl for_loop>
}

// --- module impl definitions ---

/// Implementation for type `Values`.
impl<'a> Values<'a> {
  /// Get key of Object style loop for current iteration
  ///
  ///  * `i` - Index of loop
  ///  * _return_ - Key for current iteration
  ///
  #[inline]
  pub fn current_key(&self, i: usize) -> RefOrOwned<'a, Value> {
    // custom <fn values_current_key>

    match self {
      Values::ArrayValues { values } => panic!("No key in array list"),
      Values::ObjectValues { values } => values.get(i).expect("Value").0.clone(),
    }

    // end <fn values_current_key>
  }

  /// Get value of Object style loop for current iteration
  ///
  ///  * `i` - Index of loop
  ///  * _return_ - Value for current iteration
  ///
  #[inline]
  pub fn current_value(&self, i: usize) -> RefOrOwned<'a, Value> {
    // custom <fn values_current_value>

    match self {
      Values::ArrayValues { values } => {
        if let Some(array) = values.get_ref() {
          RefOrOwned::from_borrow(array.as_array().expect("Is array").get(i).expect("Value"))
        } else {
          RefOrOwned::from_owned(
            values
              .as_array()
              .expect("Is array")
              .get(i)
              .expect("Value")
              .clone(),
          )
        }
      }
      Values::ObjectValues { values } => values.get(i).expect("Value").1.clone(),
    }
    // end <fn values_current_value>
  }

  // custom <impl values>
  // end <impl values>
}
