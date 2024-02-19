use std::borrow::Cow;
use std::collections::HashMap;

use serde_json::Value;

use crate::context::dotted_pointer;
use crate::renderer::for_loop::ForLoop;
use crate::template::Template;

pub type Val<'a> = Cow<'a, Value>;
pub type FrameContext<'a> = HashMap<&'a str, Val<'a>>;

/// Gets a value within a value by pointer, keeping lifetime
#[inline]
pub fn value_by_pointer<'a>(pointer: &str, val: &Val<'a>) -> Option<Val<'a>> {
    match *val {
        Cow::Borrowed(r) => dotted_pointer(r, pointer).map(Cow::Borrowed),
        Cow::Owned(ref r) => dotted_pointer(r, pointer).map(|found| Cow::Owned(found.clone())),
    }
}

/// Enumerates the types of stack frames
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FrameType {
    /// Original frame
    Origin,
    /// New frame for macro call
    Macro,
    /// New frame for for loop
    ForLoop,
    /// Include template
    Include,
}

/// Entry in the stack frame
#[derive(Debug)]
pub struct StackFrame<'a> {
    /// Type of stack frame
    pub kind: FrameType,
    /// Frame name for context/debugging
    pub name: &'a str,
    /// Assigned value (via {% set ... %}, {% for ... %}, {% namespace::macro(a=a, b=b) %})
    ///
    /// - {% set ... %} adds to current frame_context
    /// - {% for ... %} builds frame_context before iteration
    /// - {% namespace::macro(a=a, b=b)} builds frame_context before invocation
    context: FrameContext<'a>,
    /// Active template for frame
    pub active_template: &'a Template,
    /// `ForLoop` if frame is for a for loop
    pub for_loop: Option<ForLoop<'a>>,
    /// Macro namespace if MacroFrame
    pub macro_namespace: Option<&'a str>,
}

impl<'a> StackFrame<'a> {
    pub fn new(kind: FrameType, name: &'a str, tpl: &'a Template) -> Self {
        StackFrame {
            kind,
            name,
            context: FrameContext::new(),
            active_template: tpl,
            for_loop: None,
            macro_namespace: None,
        }
    }

    pub fn new_for_loop(name: &'a str, tpl: &'a Template, for_loop: ForLoop<'a>) -> Self {
        StackFrame {
            kind: FrameType::ForLoop,
            name,
            context: FrameContext::new(),
            active_template: tpl,
            for_loop: Some(for_loop),
            macro_namespace: None,
        }
    }

    pub fn new_macro(
        name: &'a str,
        tpl: &'a Template,
        macro_namespace: &'a str,
        context: FrameContext<'a>,
    ) -> Self {
        StackFrame {
            kind: FrameType::Macro,
            name,
            context,
            active_template: tpl,
            for_loop: None,
            macro_namespace: Some(macro_namespace),
        }
    }

    pub fn new_include(name: &'a str, tpl: &'a Template) -> Self {
        StackFrame {
            kind: FrameType::Include,
            name,
            context: FrameContext::new(),
            active_template: tpl,
            for_loop: None,
            macro_namespace: None,
        }
    }

    /// Finds a value in the stack frame.
    /// Looks first in `frame_context`, then compares to for_loop key_name and value_name.
    pub fn find_value(&self, key: &str) -> Option<Val<'a>> {
        self.find_value_in_frame(key).or_else(|| self.find_value_in_for_loop(key))
    }

    /// Finds a value in `frame_context`.
    pub fn find_value_in_frame(&self, key: &str) -> Option<Val<'a>> {
        if let Some(dot) = key.find('.') {
            if dot < key.len() + 1 {
                if let Some(found_value) =
                    self.context.get(&key[0..dot]).map(|v| value_by_pointer(&key[dot + 1..], v))
                {
                    return found_value;
                }
            }
        } else if let Some(found) = self.context.get(key) {
            return Some(found.clone());
        }

        None
    }
    /// Finds a value in the `for_loop` if there is one
    pub fn find_value_in_for_loop(&self, key: &str) -> Option<Val<'a>> {
        if let Some(ref for_loop) = self.for_loop {
            // 1st case: the variable is the key of a KeyValue for loop
            if for_loop.is_key(key) {
                return Some(Cow::Owned(Value::String(for_loop.get_current_key())));
            }

            let (real_key, tail) = if let Some(tail_pos) = key.find('.') {
                (&key[..tail_pos], &key[tail_pos + 1..])
            } else {
                (key, "")
            };

            // 2nd case: one of Tera loop built-in variable
            if real_key == "loop" {
                match tail {
                    "index" => {
                        return Some(Cow::Owned(Value::Number((for_loop.current + 1).into())));
                    }
                    "index0" => {
                        return Some(Cow::Owned(Value::Number(for_loop.current.into())));
                    }
                    "first" => {
                        return Some(Cow::Owned(Value::Bool(for_loop.current == 0)));
                    }
                    "last" => {
                        return Some(Cow::Owned(Value::Bool(
                            for_loop.current == for_loop.len() - 1,
                        )));
                    }
                    _ => return None,
                };
            }

            // Last case: the variable is/starts with the value name of the for loop
            // The `set` case will have been taken into account before

            // Exact match to the loop value and no tail
            if key == for_loop.value_name {
                return Some(for_loop.get_current_value());
            }

            if real_key == for_loop.value_name && !tail.is_empty() {
                return value_by_pointer(tail, &for_loop.get_current_value());
            }
        }

        None
    }

    /// Insert a value in the context
    pub fn insert(&mut self, key: &'a str, value: Val<'a>) {
        self.context.insert(key, value);
    }

    /// Context is cleared on each loop
    pub fn clear_context(&mut self) {
        if self.for_loop.is_some() {
            self.context.clear();
        }
    }

    pub fn context_owned(&self) -> HashMap<String, Value> {
        let mut context = HashMap::new();

        for (key, val) in &self.context {
            context.insert((*key).to_string(), val.clone().into_owned());
        }

        context
    }
}
