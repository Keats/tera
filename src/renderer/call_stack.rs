//! Defines entries in the macro call stack

// --- module use statements ---

use context::get_json_pointer;
use errors::{Result, ResultExt};
use renderer::context::Context;
use renderer::for_loop::{ForLoop, ForLoopState, Values};
use renderer::ref_or_owned::RefOrOwned;
use serde_json::{to_value, Value};
use std::collections::HashMap;
use template::Template;

// --- module type aliases ---

pub type FrameContext<'a> = HashMap<&'a str, RefOrOwned<'a, Value>>;

// --- module enum definitions ---

/// Enumerates the types of stack frames
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FrameType {
  /// Original frame
  TopFrame,
  /// New frame for macro call
  MacroFrame,
  /// New frame for for loop
  ForLoopFrame,
}

// --- module struct definitions ---

/// Entry in the stack frame
#[derive(Debug)]
pub struct StackFrame<'a> {
  /// Type of stack frame
  frame_type: FrameType,
  /// Frame name for context/debugging
  frame_name: &'a str,
  /// Assigned value (via {% set ... %}, {% for ... %}, {% namespace::macro(a=a, b=b) %})
  ///
  /// - {% set ... %} adds to current frame_context
  /// - {% for ... %} builds frame_context before iteration
  /// - {% namespace::macro(a=a, b=b)} builds frame_context before invocation
  ///
  frame_context: FrameContext<'a>,
  /// Active template for frame
  active_template: &'a Template,
  /// `ForLoop` if frame is for a for loop
  for_loop: Option<ForLoop<'a>>,
}
/// Implementation for type `StackFrame`.
impl<'a> StackFrame<'a> {
  /// Finds a value in the stack frame.
  ///
  /// Looks first in `frame_context`, then compares to for_loop key_name and value_name.
  ///
  ///  * `key` - Key to find
  ///  * _return_ - Found value or `None`
  ///
  #[inline]
  pub fn find_value(self: &Self, key: &'a str) -> Option<RefOrOwned<'a, Value>> {
    // custom <fn stack_frame_find_value>

    if let Some(dot) = key.find('.') {
      if dot < key.len() + 1 {
        if let Some(found_value) = self
          .frame_context
          .get(&key[0..dot])
          .map(|v| value_by_pointer(&key[dot + 1..], v))
        {
          return found_value;
        }
      } else {
        warn!("Invalid variable lookup ending in `.` -> `{}`", key);
      }
    } else if let Some(found) = self.frame_context.get(key) {
      return Some(found.clone());
    }

    if let Some(for_loop) = &self.for_loop {
      match key {
        "loop.index" => {
          return Some(RefOrOwned::from_owned(
            to_value(&(for_loop.current() + 1)).expect("usize to_value"),
          ));
        }
        "loop.index0" => {
          return Some(RefOrOwned::from_owned(
            to_value(&for_loop.current()).expect("usize to_value"),
          ));
        }
        "loop.first" => {
          return Some(RefOrOwned::from_owned(
            to_value(&(for_loop.current() == 0)).expect("bool to_value"),
          ));
        }
        "loop.last" => {
          return Some(RefOrOwned::from_owned(
            to_value(&(for_loop.current() == for_loop.len() - 1)).expect("bool to_value"),
          ));
        }
        _ => (),
      }

      let value_name = for_loop.value_name();
      if key.starts_with(value_name) {
        let current_value = for_loop.current_value().clone();
        if key.len() == value_name.len() {
          return Some(current_value);
        } else {
          if key.as_bytes()[value_name.len()] == ".".as_bytes()[0] {
            return value_by_pointer(key.split_at(value_name.len() + 1).1, &current_value);
          }
        }
      }

      if let Some(for_loop_key) = for_loop.key_name() {
        if key.starts_with(for_loop_key) {
          let current_key = for_loop.current_key().clone();
          if key.len() == for_loop_key.len() {
            return Some(current_key);
          } else if key.as_bytes()[for_loop_key.len()] == ".".as_bytes()[0] {
            return value_by_pointer(key.split_at(value_name.len() + 1).1, &current_key);
          }
        }
      }
    }

    None

    // end <fn stack_frame_find_value>
  }

  // custom <impl stack_frame>
  // end <impl stack_frame>
}

/// Contains the stack of frames
#[derive(Debug)]
pub struct CallStack<'a> {
  /// The stack of frames
  stack: Vec<StackFrame<'a>>,
  /// User supplied context for the render
  context: Context<'a>,
}
/// Implementation for type `CallStack`.
impl<'a> CallStack<'a> {
  /// Create empty `CallStack` with provided context
  ///
  ///  * `context` - User supplied context for the render
  ///  * `active_template` - The active template for top frame
  ///  * _return_ - New empty `CallStack`
  ///
  #[inline]
  pub fn from_context(context: Context<'a>, active_template: &'a Template) -> CallStack<'a> {
    CallStack {
      stack: vec![StackFrame {
        frame_type: FrameType::TopFrame,
        frame_name: "__TOP__",
        frame_context: FrameContext::new(),
        active_template,
        for_loop: None,
      }],
      context,
    }
  }

  /// Pushes a new `StackFrame` to the stack
  ///
  ///  * `frame_name` - Name for context for logging
  ///  * `for_loop` - `ForLoop` if frame is for loop
  ///
  #[inline]
  pub fn push_for_loop_frame(&mut self, frame_name: &'a str, for_loop: ForLoop<'a>) -> () {
    let active_template = self.stack.last().expect("Stack frame").active_template;
    self.stack.push(StackFrame {
      frame_type: FrameType::ForLoopFrame,
      frame_name,
      frame_context: HashMap::new(),
      active_template,
      for_loop: Some(for_loop),
    })
  }

  /// Pushes a new `StackFrame` to the stack
  ///
  ///  * `frame_name` - Name for context for logging
  ///  * `frame_context` - Context for the frame
  ///  * `active_template` - Template with macro definition and the new *active* template
  ///
  #[inline]
  pub fn push_macro_frame(
    &mut self,
    frame_name: &'a str,
    frame_context: FrameContext<'a>,
    active_template: &'a Template,
  ) -> () {
    self.stack.push(StackFrame {
      frame_type: FrameType::MacroFrame,
      frame_name,
      frame_context,
      active_template,
      for_loop: None,
    })
  }

  /// Pops latest `StackFrame`
  ///
  #[inline]
  pub fn pop_frame(&mut self) -> () {
    debug_assert!(self.stack.last().expect("Last Frame").frame_type != FrameType::TopFrame);
    self.stack.pop().expect("Last Frame");
  }

  /// Finds a value in the stack frame or prior stack frames.
  ///
  /// Walks assignments of for loops in current and `ForLoopFrame`s.
  /// Stops walk at change of `FrameType`
  ///
  ///  * `key` - Key to find
  ///  * _return_ - Found value or `None`
  ///
  #[inline]
  pub fn find_value(self: &Self, key: &'a str) -> Option<RefOrOwned<'a, Value>> {
    // custom <fn call_stack_find_value>

    let first_frame_type = self.stack.last().as_ref().expect("Frame exists").frame_type;

    // When searching up stack, if current frame type is `TopFrame`
    // or `MacroFrame` - end after looking in current frame

    for stack_frame in self.stack.iter().rev() {
      // Look in assignments
      let found = stack_frame.find_value(key);
      if found.is_some() {
        return found;
      }

      // If just looked in assignments of macro or top, no point in continuing.
      // If top it is end of line, if macro call it's parent frame is not visible
      let frame_type = stack_frame.frame_type;
      if frame_type == FrameType::MacroFrame || frame_type == FrameType::TopFrame {
        break;
      }
    }

    // Not in stack frame, look in user supplied context
    if let Some(dot) = key.find('.') {
      return self
        .context
        .find_value_by_pointer(&get_json_pointer(key))
        .map(|v| RefOrOwned::from_borrow(v));
    } else if let Some(value) = self.context.find_value(key) {
      return Some(RefOrOwned::from_borrow(value));
    }

    None

    // end <fn call_stack_find_value>
  }

  /// Add an assignment value (via {% set ... %})
  ///
  ///  * `key` - Identifier of the assignment
  ///  * `value` - Value of assignment
  ///
  #[inline]
  pub fn add_assignment(self: &mut Self, key: &'a str, value: RefOrOwned<'a, Value>) -> () {
    self.current_frame_mut().frame_context.insert(key, value);
  }

  /// Returns mutable reference to current `StackFrame`
  ///
  ///  * _return_ - Current stack frame
  ///
  #[inline]
  pub fn current_frame_mut(self: &mut Self) -> &mut StackFrame<'a> {
    self.stack.last_mut().expect("Current frame")
  }

  /// Returns mutable reference to current `StackFrame`
  ///
  ///  * _return_ - Current stack frame
  ///
  #[inline]
  pub fn current_frame(self: &Self) -> &StackFrame<'a> {
    self.stack.last().expect("Current frame")
  }

  /// Gets reference to current template
  ///
  ///  * _return_ - The current template in template stack
  ///
  #[inline]
  pub fn active_template(&self) -> &'a Template {
    self.current_frame().active_template
  }

  /// Breaks current for loop
  ///
  ///  * _return_ - Fails if not in for loop
  ///
  #[inline]
  pub fn break_for_loop(self: &mut Self) -> Result<()> {
    // custom <fn call_stack_break_for_loop>

    match &mut self.current_frame_mut().for_loop {
      Some(for_loop) => {
        for_loop.break_loop();
        Ok(())
      }
      None => bail!("Attempted `break` while not in `for loop`"),
    }

    // end <fn call_stack_break_for_loop>
  }

  /// Continues current for loop
  ///
  ///  * _return_ - Fails if not in for loop
  ///
  #[inline]
  pub fn increment_for_loop(self: &mut Self) -> Result<()> {
    // custom <fn call_stack_increment_for_loop>

    match &mut self.current_frame_mut().for_loop {
      Some(for_loop) => {
        for_loop.increment();
        Ok(())
      }
      None => bail!("Attempted `increment` while not in `for loop`"),
    }
    // end <fn call_stack_increment_for_loop>
  }

  /// Continues current for loop
  ///
  ///  * _return_ - Fails if not in for loop
  ///
  #[inline]
  pub fn continue_for_loop(self: &mut Self) -> Result<()> {
    // custom <fn call_stack_continue_for_loop>

    match &mut self.current_frame_mut().for_loop {
      Some(for_loop) => {
        for_loop.continue_loop();
        Ok(())
      }
      None => bail!("Attempted `continue` while not in `for loop`"),
    }

    // end <fn call_stack_continue_for_loop>
  }

  /// True if should break body
  ///
  ///  * _return_ - If for loop and and in continue or break state
  ///
  #[inline]
  pub fn should_break_body(&self) -> bool {
    match &self.current_frame().for_loop {
      Some(for_loop) => {
        for_loop.for_loop_state() == ForLoopState::Break
          || for_loop.for_loop_state() == ForLoopState::Continue
      }
      None => false,
    }
  }

  /// Gets text display of all context data
  ///
  ///  * _return_ - Display formatted context
  ///
  pub fn debug_context(&self) -> String {
    // custom <fn call_stack_debug_context>

    let mut result = String::new();

    for stack_frame in self.stack.iter().rev() {
      result.push_str(&format!(
        "
---- Frame({}) ----
{:#?}
---- Begin For Loop ----
{:#?}
---- End For Loop ----
",
        stack_frame.frame_name, stack_frame.frame_context, stack_frame.for_loop
      ));
    }

    result

    // end <fn call_stack_debug_context>
  }

  // custom <impl call_stack>
  // end <impl call_stack>
}

// --- module function definitions ---

/// Gets a value within a value by pointer, keeping lifetime
///
///  * `pointer_path` - Pointer path to find value in object
///  * `ref_or_owned` - Object to point into
///  * _return_ - Referred to object or None
///
#[inline]
pub fn value_by_pointer<'a>(
  pointer_path: &str,
  ref_or_owned: &RefOrOwned<'a, Value>,
) -> Option<RefOrOwned<'a, Value>> {
  // custom <fn value_by_pointer>

  if let Some(borrow) = ref_or_owned.get_ref() {
    borrow
      .pointer(&get_json_pointer(pointer_path))
      .map(|found| RefOrOwned::from_borrow(found))
  } else {
    ref_or_owned
      .pointer(&get_json_pointer(pointer_path))
      .map(|found| RefOrOwned::from_owned(found.clone()))
  }

  // end <fn value_by_pointer>
}
