use crate::args::{ArgFromValue, Kwargs};
use crate::errors::TeraResult;
use crate::filters::StoredFilter;
use crate::parsing::Chunk;
use crate::vm::for_loop::ForLoop;
use crate::vm::stack::Stack;
use crate::{Context, HashMap, Value};

use std::borrow::Cow;
use std::collections::BTreeMap;

/// Special string indicating request to dump context
pub(crate) static MAGICAL_DUMP_VAR: &str = "__tera_context";

/// The state of the interpreter.
/// We pass it around rather than put it on the VM to avoid multiple borrow issues
/// when dealing with inheritance.
#[derive(Debug)]
pub struct State<'tera> {
    pub(crate) stack: Stack,
    /// It can be None for things like tests as we don't expose Chunk outside of the crate
    pub(crate) chunk: Option<&'tera Chunk>,
    pub(crate) for_loops: Vec<ForLoop>,
    /// Any variables with {% set %} outside a for loop or {% set_global %} will be stored here
    /// Locals set in a for loop are set in `for_loops`
    set_variables: BTreeMap<String, Value>,
    pub(crate) context: &'tera Context,
    /// The global context from Tera, checked after user context
    pub(crate) global_context: Option<&'tera Context>,
    /// To handle the capture instructions
    pub(crate) capture_buffers: Vec<Vec<u8>>,
    /// Scratch buffer for escaping output to avoid per-write allocations
    pub(crate) escape_buffer: Vec<u8>,
    /// Used in includes only
    pub(crate) include_parent: Option<&'tera State<'tera>>,

    /// (block name, (all_chunks, level))
    pub(crate) blocks: BTreeMap<&'tera str, (Vec<&'tera Chunk>, usize)>,
    pub(crate) current_block_name: Option<&'tera str>,
    /// Reference to registered filters for calling filters from within filters (e.g., map filter)
    pub(crate) filters: Option<&'tera HashMap<Cow<'static, str>, StoredFilter>>,
}

impl<'t> State<'t> {
    pub(crate) fn new_with_chunk(context: &'t Context, chunk: &'t Chunk) -> Self {
        let mut s = Self::new(context);
        s.chunk = Some(chunk);
        s
    }

    pub fn new(context: &'t Context) -> Self {
        Self {
            stack: Stack::new(),
            for_loops: Vec::with_capacity(4),
            set_variables: BTreeMap::new(),
            context,
            global_context: None,
            chunk: None,
            capture_buffers: Vec::with_capacity(4),
            escape_buffer: Vec::with_capacity(128),
            include_parent: None,
            blocks: BTreeMap::new(),
            current_block_name: None,
            filters: None,
        }
    }

    pub(crate) fn store_local(&mut self, name: &str, value: Value) {
        if let Some(forloop) = self.for_loops.last_mut() {
            forloop.store(name, value);
        } else {
            self.store_global(name, value);
        }
    }

    pub(crate) fn store_global(&mut self, name: &str, value: Value) {
        self.set_variables.insert(name.to_string(), value);
    }

    /// Loads the value with the current name on the stack
    /// It goes in the following order for scopes:
    /// 1. All loops from the inner to the outer
    /// 2. set_variables
    /// 3. self.context (user context)
    /// 4. self.global_context (Tera's global context)
    /// 5. include_parent or return Value::Undefined
    pub(crate) fn get_value(&self, name: &str) -> Value {
        for forloop in self.for_loops.iter().rev() {
            if let Some(v) = forloop.get(name) {
                return v;
            }
        }

        if let Some(val) = self.set_variables.get(name) {
            return val.clone();
        }

        if let Some(val) = self.context.data.get(name) {
            return val.clone();
        }

        if let Some(global) = self.global_context
            && let Some(val) = global.data.get(name)
        {
            return val.clone();
        }

        if let Some(parent) = self.include_parent {
            parent.get_value(name)
        } else {
            Value::undefined()
        }
    }

    /// Get a variable from the context by name and convert it to the specified type.
    ///
    /// Returns `Ok(None)` if the variable is not defined (undefined).
    /// Returns an error if the variable exists but cannot be converted to the target type.
    pub fn get<T>(&self, name: &str) -> TeraResult<Option<T>>
    where
        for<'a> T: ArgFromValue<'a, Output = T>,
    {
        let value = self.get_value(name);
        if value.is_undefined() {
            Ok(None)
        } else {
            T::from_value(&value).map(Some)
        }
    }

    pub(crate) fn dump_context(&self) -> Value {
        let mut context = crate::HashMap::new();
        // Add global context first (lowest priority)
        if let Some(global) = self.global_context {
            for (k, v) in &global.data {
                context.insert(k.to_string(), v.clone());
            }
        }
        // User context overrides global
        for (k, v) in &self.context.data {
            context.insert(k.to_string(), v.clone());
        }
        // set_variables override user context
        context.extend(self.set_variables.clone());

        for forloop in &self.for_loops {
            context.extend(forloop.context.clone());
        }

        context.into()
    }

    pub(crate) fn load_name(&mut self, name: &str, span_idx: u32) {
        if name == MAGICAL_DUMP_VAR {
            self.stack.push(self.dump_context(), None);
        } else {
            self.stack
                .push(self.get_value(name), Some(span_idx..=span_idx));
        }
    }

    /// Call a filter by name. Used by filters like `map` that need to apply other filters.
    pub fn call_filter(&self, name: &str, value: &Value, kwargs: Kwargs) -> TeraResult<Value> {
        match self.filters.and_then(|f| f.get(name)) {
            Some(filter) => {
                let val = filter.call(value, kwargs, self)?;
                Ok(if filter.is_safe() {
                    val.mark_safe()
                } else {
                    val
                })
            }
            None => Err(crate::errors::Error::message(format!(
                "Filter `{name}` is not registered"
            ))),
        }
    }

    /// Returns a sorted list of all available variable names in the current scope.
    /// Used for error messages only.
    pub(crate) fn available_variables(&self) -> Vec<String> {
        let mut vars = std::collections::BTreeSet::new();

        if let Some(global) = self.global_context {
            for k in global.data.keys() {
                vars.insert(k.to_string());
            }
        }

        for k in self.context.data.keys() {
            vars.insert(k.to_string());
        }

        for k in self.set_variables.keys() {
            vars.insert(k.clone());
        }

        for forloop in &self.for_loops {
            for k in forloop.context.keys() {
                vars.insert(k.clone());
            }
        }

        vars.into_iter().collect()
    }
}
