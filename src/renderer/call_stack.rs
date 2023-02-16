use std::borrow::Cow;
use std::collections::HashMap;

use serde_json::{to_value, Value};

use crate::context::dotted_pointer;
use crate::errors::{Error, Result};
use crate::renderer::for_loop::{ForLoop, ForLoopState};
use crate::renderer::stack_frame::{FrameContext, FrameType, StackFrame, Val};
use crate::template::Template;
use crate::Context;

/// Contains the user data and allows no mutation
#[derive(Debug)]
pub struct UserContext<'a> {
    /// Read-only context
    inner: &'a Context,
}

impl<'a> UserContext<'a> {
    /// Create an immutable user context to be used in the call stack
    pub fn new(context: &'a Context) -> Self {
        UserContext { inner: context }
    }

    pub fn find_value(&self, key: &str) -> Option<&'a Value> {
        self.inner.get(key)
    }

    pub fn find_value_by_dotted_pointer(&self, pointer: &str) -> Option<&'a Value> {
        let root = pointer.split('.').next().unwrap().replace("~1", "/").replace("~0", "~");
        let rest = &pointer[root.len() + 1..];
        self.inner.get(&root).and_then(|val| dotted_pointer(val, rest))
    }
}

/// Contains the stack of frames
#[derive(Debug)]
pub struct CallStack<'a> {
    /// The stack of frames
    stack: Vec<StackFrame<'a>>,
    /// User supplied context for the render
    context: UserContext<'a>,
}

impl<'a> CallStack<'a> {
    /// Create the initial call stack
    pub fn new(context: &'a Context, template: &'a Template) -> CallStack<'a> {
        CallStack {
            stack: vec![StackFrame::new(FrameType::Origin, "ORIGIN", template)],
            context: UserContext::new(context),
        }
    }

    pub fn push_for_loop_frame(&mut self, name: &'a str, for_loop: ForLoop<'a>) {
        let tpl = self.stack.last().expect("Stack frame").active_template;
        self.stack.push(StackFrame::new_for_loop(name, tpl, for_loop));
    }

    pub fn push_macro_frame(
        &mut self,
        namespace: &'a str,
        name: &'a str,
        context: FrameContext<'a>,
        tpl: &'a Template,
    ) {
        self.stack.push(StackFrame::new_macro(name, tpl, namespace, context));
    }

    pub fn push_include_frame(&mut self, name: &'a str, tpl: &'a Template) {
        self.stack.push(StackFrame::new_include(name, tpl));
    }

    /// Returns mutable reference to global `StackFrame`
    /// i.e gets first stack outside current for loops
    pub fn global_frame_mut(&mut self) -> &mut StackFrame<'a> {
        if self.current_frame().kind == FrameType::ForLoop {
            for stack_frame in self.stack.iter_mut().rev() {
                // walk up the parent stacks until we meet the current template
                if stack_frame.kind != FrameType::ForLoop {
                    return stack_frame;
                }
            }
            unreachable!("Global frame not found when trying to break out of for loop");
        } else {
            // Macro, Origin, or Include
            self.current_frame_mut()
        }
    }

    /// Returns mutable reference to current `StackFrame`
    pub fn current_frame_mut(&mut self) -> &mut StackFrame<'a> {
        self.stack.last_mut().expect("No current frame exists")
    }

    /// Returns immutable reference to current `StackFrame`
    pub fn current_frame(&self) -> &StackFrame<'a> {
        self.stack.last().expect("No current frame exists")
    }

    /// Pop the last frame
    pub fn pop(&mut self) {
        self.stack.pop().expect("Mistakenly popped Origin frame");
    }

    pub fn lookup(&self, key: &str) -> Option<Val<'a>> {
        for stack_frame in self.stack.iter().rev() {
            let found = stack_frame.find_value(key);
            if found.is_some() {
                return found;
            }

            // If we looked in a macro or origin frame, no point continuing
            // Origin is the last one and macro frame don't have access to parent frames
            if stack_frame.kind == FrameType::Macro || stack_frame.kind == FrameType::Origin {
                break;
            }
        }

        // Not in stack frame, look in user supplied context
        if key.contains('.') {
            return self.context.find_value_by_dotted_pointer(key).map(Cow::Borrowed);
        } else if let Some(value) = self.context.find_value(key) {
            return Some(Cow::Borrowed(value));
        }

        None
    }

    /// Add an assignment value (via {% set ... %} and {% set_global ... %} )
    pub fn add_assignment(&mut self, key: &'a str, global: bool, value: Val<'a>) {
        if global {
            self.global_frame_mut().insert(key, value);
        } else {
            self.current_frame_mut().insert(key, value);
        }
    }

    /// Breaks current for loop
    pub fn break_for_loop(&mut self) -> Result<()> {
        match self.current_frame_mut().for_loop {
            Some(ref mut for_loop) => {
                for_loop.break_loop();
                Ok(())
            }
            None => Err(Error::msg("Attempted `break` while not in `for loop`")),
        }
    }

    /// Continues current for loop
    pub fn increment_for_loop(&mut self) -> Result<()> {
        let frame = self.current_frame_mut();
        frame.clear_context();
        match frame.for_loop {
            Some(ref mut for_loop) => {
                for_loop.increment();
                Ok(())
            }
            None => Err(Error::msg("Attempted `increment` while not in `for loop`")),
        }
    }

    /// Continues current for loop
    pub fn continue_for_loop(&mut self) -> Result<()> {
        match self.current_frame_mut().for_loop {
            Some(ref mut for_loop) => {
                for_loop.continue_loop();
                Ok(())
            }
            None => Err(Error::msg("Attempted `continue` while not in `for loop`")),
        }
    }

    /// True if should break body, applicable to `break` and `continue`
    pub fn should_break_body(&self) -> bool {
        match self.current_frame().for_loop {
            Some(ref for_loop) => {
                for_loop.state == ForLoopState::Break || for_loop.state == ForLoopState::Continue
            }
            None => false,
        }
    }

    /// True if should break loop, applicable to `break` only
    pub fn should_break_for_loop(&self) -> bool {
        match self.current_frame().for_loop {
            Some(ref for_loop) => for_loop.state == ForLoopState::Break,
            None => false,
        }
    }

    /// Grab the current frame template
    pub fn active_template(&self) -> &'a Template {
        self.current_frame().active_template
    }

    pub fn current_context_cloned(&self) -> Value {
        let mut context = HashMap::new();

        // Go back the stack in reverse to see what we have access to
        for frame in self.stack.iter().rev() {
            context.extend(frame.context_owned());
            if let Some(ref for_loop) = frame.for_loop {
                context.insert(
                    for_loop.value_name.to_string(),
                    for_loop.get_current_value().into_owned(),
                );
                if for_loop.is_key_value() {
                    context.insert(
                        for_loop.key_name.clone().unwrap(),
                        Value::String(for_loop.get_current_key()),
                    );
                }
            }
            // Macros don't have access to the user context, we're done
            if frame.kind == FrameType::Macro {
                return to_value(&context).unwrap();
            }
        }

        // If we are here we take the user context
        // and add the values found in the stack to it.
        // We do it this way as we can override global variable temporarily in forloops
        let mut new_ctx = self.context.inner.clone();
        for (key, val) in context {
            new_ctx.insert(key, &val)
        }
        new_ctx.into_json()
    }
}
