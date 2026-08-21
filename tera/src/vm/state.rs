use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::args::{ArgFromValue, Kwargs};
use crate::errors::TeraResult;
use crate::filters::StoredFilter;
use crate::parsing::Chunk;
use crate::vm::for_loop::ForLoop;
use crate::vm::stack::Stack;
use crate::{Context, Error, EscapeFn, HashMap, Tera, Value, escape_html};

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
    /// Which block we are asked to render
    pub(crate) capture_block: Option<&'tera str>,
    /// The buffer just for the one block we want to return
    pub(crate) block_buffer: Vec<u8>,

    /// (block name, all_chunks, level).
    pub(crate) blocks: Vec<(&'tera str, &'tera Vec<Chunk>, usize)>,
    pub(crate) current_block_name: Option<&'tera str>,
    /// Reference to registered filters for calling filters from within filters (e.g., map filter)
    pub(crate) filters: Option<&'tera HashMap<Cow<'static, str>, StoredFilter>>,
    /// The escape fn defined in the Tera instance
    pub(crate) escape_fn: EscapeFn,
    /// Whether the current content has autoescape enabled or not
    pub(crate) autoescaping_enabled: bool,
}

impl<'t> State<'t> {
    pub(crate) fn new_with_chunk(
        tera: &'t Tera,
        context: &'t Context,
        chunk: &'t Chunk,
        autoescape_enabled: bool,
    ) -> Self {
        let mut s = Self::new(context);
        s.chunk = Some(chunk);
        s.escape_fn = tera.escape_fn;
        s.filters = Some(&tera.filters);
        s.autoescaping_enabled = autoescape_enabled;
        s
    }

    /// Creates a new state from a `Context`.
    /// Public since it's needed to test filters/fns/tests but there are no filters registered to it.
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
            capture_block: None,
            block_buffer: Vec::new(),
            blocks: Vec::new(),
            current_block_name: None,
            filters: None,
            escape_fn: escape_html,
            autoescaping_enabled: true,
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
    /// 3. include_parent
    /// 4. self.context (user context)
    /// 5. self.global_context (Tera's global context) or return Value::Undefined
    pub(crate) fn get_value(&self, name: &str) -> Value {
        for forloop in self.for_loops.iter().rev() {
            if let Some(v) = forloop.get(name) {
                return v;
            }
        }

        if let Some(val) = self.set_variables.get(name) {
            return val.clone();
        }

        if let Some(parent) = self.include_parent {
            let val = parent.get_value(name);
            if !val.is_undefined() {
                return val;
            }
        }

        if let Some(val) = self.context.data.get(name) {
            return val.clone();
        }

        if let Some(global) = self.global_context
            && let Some(val) = global.data.get(name)
        {
            return val.clone();
        }

        Value::undefined()
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

    /// Whether autoescaping is enabled for the current content
    pub fn autoescaping_enabled(&self) -> bool {
        self.autoescaping_enabled
    }

    /// Escapes a string using the escape fn defined in the Tera struct, but only
    /// if the current content needs escaping and is not already safe.
    /// If you need unconditional escaping, use `escape`
    pub fn escape_if_needed(&self, value: &Value) -> TeraResult<String> {
        if value.is_undefined() {
            return Err(Error::message("Tried to escape an undefined value"));
        }

        let mut formatted = Vec::new();
        value.format(&mut formatted)?;
        let formatted =
            String::from_utf8(formatted).expect("Value::format only writes valid UTF-8");

        if !self.autoescaping_enabled || value.is_safe() {
            return Ok(formatted);
        }

        self.escape(&formatted)
    }

    /// Escapes a string using the escape fn defined in the Tera struct
    pub fn escape(&self, input: &str) -> TeraResult<String> {
        let mut buf: Vec<u8> = Vec::with_capacity(input.len());
        (self.escape_fn)(input, &mut buf)?;
        let escaped = String::from_utf8(buf).map_err(|e| {
            Error::message(format!(
                "String `{input}` could not be converted to UTF-8 after escaping: {e}"
            ))
        })?;

        Ok(escaped)
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
            self.stack.push(self.dump_context(), span_idx..=span_idx);
        } else {
            self.stack.push(self.get_value(name), span_idx..=span_idx);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tera;

    #[test]
    fn can_handle_escaping() {
        let mut tera = Tera::default();
        tera.register_function("force_esc", |kwargs: Kwargs, state: &State| {
            let text = kwargs.must_get::<&str>("text")?;
            state.escape(text)
        });
        tera.register_function("esc_if_needed", |kwargs: Kwargs, state: &State| {
            let text = kwargs.must_get::<Value>("text")?;
            state.escape_if_needed(&text)
        });
        let mut ctx = Context::new();
        ctx.insert("name", "<script>");

        // First with autoescape disabled
        tera.autoescape_on(vec![".txt"]);
        tera.add_raw_template(
            "tpl.html",
            "{{ force_esc(text=name) | safe }} - {{esc_if_needed(text=name)}}",
        )
        .unwrap();
        tera.add_raw_template(
            "tpl2.html",
            "{% set safe_name = name | safe %}{{ esc_if_needed(text=name) | safe }} - {{ esc_if_needed(text=safe_name) | safe }}",
        )
            .unwrap();
        assert_eq!(
            tera.render("tpl.html", &ctx).unwrap(),
            "&lt;script&gt; - <script>"
        );

        //  and a custom fn
        tera.set_escape_fn(|input, out| out.write_all(input.to_uppercase().as_bytes()));
        assert_eq!(
            tera.render("tpl.html", &ctx).unwrap(),
            "<SCRIPT> - <script>"
        );

        // and then autoescape enabled
        tera.reset_escape_fn();
        tera.autoescape_on(vec![".html"]);
        assert_eq!(
            tera.render("tpl2.html", &ctx).unwrap(),
            "&lt;script&gt; - <script>"
        );
    }
}
