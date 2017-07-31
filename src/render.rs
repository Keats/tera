use std::collections::{VecDeque, HashMap};

use serde_json::map::Map;
use serde_json::to_string_pretty;
use serde_json::value::{Value, to_value, Number};

use context::{ValueRender, ValueNumber, ValueTruthy, get_json_pointer};
use template::Template;
use errors::{Result, ResultExt};
use parser::{Node, Operator};
use parser::Node::*;
use tera::Tera;
use utils::escape_html;


static MAGICAL_DUMP_VAR: &'static str = "__tera_context";

#[derive(PartialEq, Debug)]
enum ForLoopKind {
    Value,
    KeyValue,
}

// we need to have some data in the renderer for when we are in a ForLoop
// For example, accessing the local variable would fail when
// looking it up in the context
#[derive(Debug)]
struct ForLoop {
    /// The key name when iterate as a Key-Value, ie in `{% for i, person in people %}` it would be `i`
    key_name: Option<String>,
    /// The value name, ie in `{% for person in people %}` it would be `person`
    value_name: String,
    /// What's the current loop index (0-indexed)
    current: usize,
    /// A list of (key, value) for the forloop. The key is `None` for `ForLoopKind::Value`
    values: Vec<(Option<String>, Value)>,
    /// Is i
    kind: ForLoopKind,
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

    pub fn new_key_value(key_name: String, value_name: &str, values: Value) -> ForLoop {
        let mut for_values = vec![];
        for (key, val) in values.as_object().unwrap() {
            for_values.push((Some(key.clone()), val.clone()));
        }

        ForLoop {
            key_name: Some(key_name),
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

#[derive(Debug)]
pub struct Renderer<'a> {
    template: &'a Template,
    context: Value,
    tera: &'a Tera,
    /// All current for loops
    for_loops: Vec<ForLoop>,
    /// Looks like Vec<filename: {macro_name: body node}>
    macros: Vec<HashMap<String, &'a HashMap<String, Node>>>,
    /// Set when rendering macros, empty if not in a macro. Loops in macros are set
    /// as part of the macro_context and not for_loops
    macro_context: Vec<(Value, Vec<ForLoop>)>,
    /// Keeps track of which namespace we're on in order to resolve the `self::` syntax
    macro_namespaces: Vec<String>,
    /// Whether this template should be escaped or not
    should_escape: bool,
    /// Used when super() is used in a block, to know where we are in our stack of
    /// definitions and for which block (block name, hierarchy level)
    blocks: Vec<(String, usize)>,
}

impl<'a> Renderer<'a> {
    pub fn new(tpl: &'a Template, tera: &'a Tera, context: Value) -> Renderer<'a> {
        let should_escape = tera.autoescape_suffixes.iter().any(|ext| {
            // We prefer a `path` if set, otherwise use the `name`
            if let Some(ref p) = tpl.path {
                return p.ends_with(ext);
            }
            tpl.name.ends_with(ext)
        });

        Renderer {
            template: tpl,
            tera: tera,
            context: context,
            for_loops: vec![],
            macros: vec![],
            macro_context: vec![],
            macro_namespaces: vec![],
            should_escape: should_escape,
            blocks: vec![],
        }
    }

    /// Lookup a variable name from the context, taking into account macros and loops
    fn lookup_variable(&self, key: &str) -> Result<Value> {
        // Differentiate between macros and general context
        let (context, for_loops) = match self.macro_context.last() {
            Some(c) => (&c.0, &c.1),
            None => (&self.context, &self.for_loops)
        };

        // Magical variable that just dumps the context
        if key == MAGICAL_DUMP_VAR {
            return Ok(to_value(
                to_string_pretty(context).expect("Couldn't serialize context for `__tera_context`")
            )?);
        }

        // small helper fn to reduce duplication code in the 3 spots in `lookup_variable` where we
        // need to actually do the variable lookup
        #[inline]
        fn find_variable(context: &Value, key: &str, tpl_name: &str) -> Result<Value> {
            match context.pointer(&get_json_pointer(key)) {
                Some(v) => Ok(v.clone()),
                None => bail!("Field `{}` not found in context while rendering '{}'", key, tpl_name)
            }
        }

        // Look in the plain context if we aren't in a for loop
        if for_loops.is_empty() {
            return find_variable(context, key, &self.template.name);
        }

        // Separates the initial key (anything before a dot) from everything after
        let (real_key, tail) = if let Some(tail_pos) = key.find('.') {
            (&key[..tail_pos], &key[tail_pos+1..])
        } else {
            (key, "")
        };

        // The variable might be from a for loop so we start from the most inner one
        for for_loop in for_loops.iter().rev() {
            // 1st case: one of Tera loop built-in variable
            if real_key == "loop" {
                match tail {
                    "index" => { return Ok(to_value(&(for_loop.current + 1))?); },
                    "index0" => { return Ok(to_value(&for_loop.current)?); },
                    "first" => { return Ok(to_value(&(for_loop.current == 0))?); },
                    "last" => { return Ok(to_value(&(for_loop.current == for_loop.len() - 1))?); },
                    _ => { bail!("Unknown loop built-in variable: {:?}", key); }
                }
            }

            // 2rd case: the variable is the key of a KeyValue for loop
            if for_loop.is_key(key) {
                return Ok(to_value(&for_loop.get_current_key())?);
            }

            // Last case: the variable starts with the value name of the for loop or has been {% set %}
            let value = if real_key == for_loop.value_name {
                for_loop.get_current_value()
            } else {
                for_loop.extra_values.get(real_key)
            };

            match value {
                Some(v) => {
                    if tail.is_empty() {
                        return Ok(v.clone());
                    }
                    // A struct or some nested structure
                    return find_variable(v, tail, &self.template.name)
                        .chain_err(|| format!("Variable lookup failed in forloop for `{}`", key));
                },
                None => ()
            };
        }

        // Gets there when looking a variable in the global context while in a forloop
        find_variable(context, key, &self.template.name)
    }

    // Gets an identifier and return its json value
    // If there is no filter, it's itself, otherwise call the filters in order
    // an return their result
    fn eval_ident(&mut self, node: &Node) -> Result<Value> {
        let (name, filters) = match *node {
            Identifier { ref name, ref filters } => (name, filters),
            _ => unreachable!(),
        };

        let mut value = self.lookup_variable(name)?;
        let mut is_safe = false;
        if let Some(ref _filters) = *filters {
            for filter in _filters {
                let (name, params) = match *filter {
                    Filter { ref name, ref params } => (name, params),
                    _ => unreachable!(),
                };

                if name == "safe" {
                    is_safe = true;
                    continue;
                }

                let filter_fn = self.tera.get_filter(name)?;
                let mut all_args = HashMap::new();
                // We don't want to escape variables used as params
                let should_escape = self.should_escape;
                self.should_escape = false;
                for (arg_name, exp) in params {
                    all_args.insert(arg_name.to_string(), self.eval_expression(exp)?);
                }
                self.should_escape = should_escape;

                value = filter_fn(value, all_args)?;
            }
        }

        // Escaping strings if wanted for that template
        if name != MAGICAL_DUMP_VAR && self.should_escape && !is_safe {
            if let Value::String(s) = value {
                value = to_value(escape_html(s.as_str()))?;
            }
        }
        Ok(value)
    }

    fn eval_math(&mut self, node: &Node) -> Result<f64> {
        match *node {
            Identifier { ref name, .. } => {
                self.eval_ident(node)?
                    .to_number()
                    .or_else(|_| Err(
                        format!(
                            "Variable `{}` was used in a math operation but is not a number", name
                        ).into()
                    ))
            },
            Int(s) => Ok(s as f64),
            Float(s) => Ok(s),
            Math { ref lhs, ref rhs, ref operator } => {
                let l = self.eval_math(lhs)?;
                let r = self.eval_math(rhs)?;
                let result = match *operator {
                    Operator::Mul => l * r,
                    Operator::Div => l / r,
                    Operator::Add => l + r,
                    Operator::Sub => l - r,
                    _ => unreachable!()
                };

                Ok(result)
            }
            Text(ref s) => bail!("Tried to do math with a String: `{}`", s),
            Bool(s) => bail!("Tried to do math with a boolean: `{}`", s),
            _ => unreachable!()
        }
    }

    fn eval_global_fn(&mut self, node: &Node) -> Result<Value> {
        match node {
            &GlobalFunctionCall { ref name, ref params } => {
                let global_fn = self.tera.get_global_function(name)?;
                let mut all_args = HashMap::new();
                // We don't want to escape variables used as params
                let should_escape = self.should_escape;
                self.should_escape = false;
                for (arg_name, exp) in params {
                    all_args.insert(arg_name.to_string(), self.eval_expression(exp)?);
                }
                self.should_escape = should_escape;

                global_fn(all_args)
            },
            _ => unreachable!()
        }
    }

    fn eval_expression(&mut self, node: &Node) -> Result<Value> {
        match node {
            &Identifier { .. } => {
                Ok(self.eval_ident(node)?)
            },
            &Logic { .. } => {
                let value = self.eval_condition(node)?;
                Ok(Value::Bool(value))
            },
            &Math { .. } => {
                let result = self.eval_math(node)?;
                match Number::from_f64(result) {
                    Some(x) => Ok(Value::Number(x)),
                    None => Ok(Value::String("NaN".to_string()))
                }
            },
            &Int(val) => {
                Ok(Value::Number(val.into()))
            },
            &Float(val) => {
                Ok(Value::Number(Number::from_f64(val).unwrap()))
            },
            &Bool(b) => {
                Ok(Value::Bool(b))
            },
            &Text(ref t) => {
                Ok(Value::String(t.to_string()))
            },
            &GlobalFunctionCall { .. } => self.eval_global_fn(node),
            _ => unreachable!()
        }
    }

    fn eval_condition(&mut self, node: &Node) -> Result<bool> {
        match node {
            &Identifier { .. } => {
                Ok(self.eval_ident(node).map(|v| v.is_truthy()).unwrap_or(false))
            },
            &Test { ref expression, ref name, ref params } => {
                let tester = self.tera.get_tester(name)?;
                let mut value_params = vec![];
                for param in params {
                    value_params.push(self.eval_expression(param)?);
                }
                tester(self.eval_expression(expression).ok(), value_params)
            },
            &Math { .. } => {
                self.eval_math(node).map(|v| v != 0.0 && !v.is_nan())
            },
            &Logic { ref lhs, ref rhs, operator } => {
                match operator {
                    Operator::Or => {
                        let result = self.eval_condition(lhs)? || self.eval_condition(rhs)?;
                        Ok(result)
                    },
                    Operator::And => {
                        let result = self.eval_condition(lhs)? && self.eval_condition(rhs)?;
                        Ok(result)
                    },
                    Operator::Gt | Operator::Gte | Operator::Lt | Operator::Lte => {
                        let l = self.eval_math(lhs)?;
                        let r = self.eval_math(rhs)?;
                        let result = match operator {
                            Operator::Gte => l >= r,
                            Operator::Gt => l > r,
                            Operator::Lte => l <= r,
                            Operator::Lt => l < r,
                            _ => unreachable!()
                        };
                        Ok(result)
                    },
                    Operator::Eq | Operator::NotEq => {
                        let mut lhs_val = self.eval_expression(lhs)?;
                        let mut rhs_val = self.eval_expression(rhs)?;

                        // Monomorphize number vals.
                        if lhs_val.is_number() || rhs_val.is_number() {
                            if !lhs_val.is_number() || !rhs_val.is_number() {
                                return Ok(false);
                            }

                            lhs_val = Value::Number(Number::from_f64(lhs_val.as_f64().unwrap()).unwrap());
                            rhs_val = Value::Number(Number::from_f64(rhs_val.as_f64().unwrap()).unwrap());
                        }

                        let result = match operator {
                            Operator::Eq => lhs_val == rhs_val,
                            Operator::NotEq => lhs_val != rhs_val,
                            _ => unreachable!()
                        };

                        Ok(result)
                    },
                    _ => unreachable!()
                }
            }
            &Not(ref n) => {
                Ok(self.eval_expression(n).map(|v| !v.is_truthy()).unwrap_or(true))
            },
            _ => unreachable!("Reached node {:?} in `eval_condition`", node)
        }
    }

    fn eval_set(&mut self, node: &Node) -> Result<()> {
        match node {
            &Set { ref name, ref value } => {
                let should_escape = self.should_escape;
                self.should_escape = false;
                let val = match **value {
                    MacroCall {..} => to_value(self.render_macro(value)?).unwrap(),
                    GlobalFunctionCall { .. } => self.eval_global_fn(value)?,
                    _ => self.eval_expression(value)?,
                };
                self.should_escape = should_escape;

                let context = match self.macro_context.last_mut() {
                    Some(c) => c.0.as_object_mut().unwrap(),
                    None => match self.for_loops.last_mut() {
                        Some(f) => &mut f.extra_values,
                        None => self.context.as_object_mut().unwrap()
                    },
                };
                context.insert(name.clone(), val);
                Ok(())
            },
            _ => unreachable!(),
        }
    }

    // eval all the values in a {{ }} block
    // Macro calls and super are NOT variable blocks in the AST, they have
    // their own nodes
    fn render_variable_block(&mut self, node: &Node) -> Result<String>  {
        match node {
            &Identifier { .. } => Ok(self.eval_ident(node)?.render()),
            &Math { .. } => Ok(self.eval_math(node)?.to_string()),
            &Text(ref s) => Ok(s.to_string()),
            _ => unreachable!("found node {:?}", node)
        }
    }

    // evaluates conditions and render bodies accordingly
    fn render_if(&mut self, condition_nodes: &VecDeque<Node>, else_node: &Option<Box<Node>>) -> Result<String> {
        let mut skip_else = false;
        let mut output = String::new();
        for node in condition_nodes {
            match node {
                &Conditional {ref condition, ref body } => {
                    if self.eval_condition(condition)? {
                        skip_else = true;
                        // Remove if/elif whitespace
                        output.push_str(self.render_node(body)?.trim_left());
                        break;
                    }
                },
                _ => unreachable!()
            }
        }

        if !skip_else {
            // Remove endif whitespace
            if let Some(ref e) = *else_node {
                // Remove else whitespace
                output.push_str(self.render_node(e)?.trim_left());
            }
        }

        // Remove endif whitespace
        Ok(output.trim_right().to_string())
    }

    fn render_for(&mut self, key_name: &Option<String>, value_name: &str, container: &Node, body: &Node) -> Result<String> {
        let container_name = match container {
            &Node::Identifier {ref name, ..} => name,
            &Node::GlobalFunctionCall {ref name, ..} => name,
            _ => unreachable!()
        };
        let container_val = self.eval_expression(container)?;

        if key_name.is_some() && !container_val.is_object() {
            bail!("Tried to iterate using key value on variable `{}`, but it isn't an object/map", container_name);
        } else if key_name.is_none() && !container_val.is_array() {
            bail!("Tried to iterate on variable `{}`, but it isn't an array", container_name);
        }
        let for_loop = if container_val.is_array() {
            ForLoop::new(value_name, container_val)
        } else {
            ForLoop::new_key_value(key_name.clone().expect("Failed to key name in loop"), value_name, container_val)
        };

        let length = for_loop.len();
        match self.macro_context.last_mut() {
            Some(m) => m.1.push(for_loop),
            None => self.for_loops.push(for_loop)
        };
        let mut output = String::new();
        for _ in 0..length {
            output.push_str(self.render_node(body)?.trim_left());
            // Safe unwrap
            match self.macro_context.last_mut() {
                Some(m) => m.1.last_mut().unwrap().increment(),
                None => self.for_loops.last_mut().unwrap().increment()
            };
        }
        match self.macro_context.last_mut() {
            Some(m) => m.1.pop(),
            None => self.for_loops.pop()
        };

        Ok(output.trim_right().to_string())
    }

    fn render_macro(&mut self, call_node: &Node) -> Result<String> {
        let (namespace, macro_name, call_params) = match *call_node {
            MacroCall { ref namespace, ref name, ref params } => (namespace, name, params),
            _ => unreachable!("Got a node other than a MacroCall when rendering a macro"),
        };

        // We need to find the active namespace in Tera if `self` is used
        // Since each macro (other than the `self` ones) pushes its own namespace
        // to the stack when being rendered, we can just lookup the last namespace that was pushed
        // to find out the active one
        let active_namespace = match namespace.as_ref() {
            "self" => {
                // TODO: handle error if we don't have a namespace
                // This can (maybe) happen when calling {{ self:: }} outside of a macro
                // This happens when calling a macro defined in the file itself without imports
                // that means macros need to be put in another file to work, which seems ok
                self.macro_namespaces
                    .last()
                    .expect("Open an issue with a template sample please (mention `self namespace macro`)!")
                    .to_string()
            },
            _ => {
                // TODO: String doesn't have Copy trait, can we avoid double cloning?
                self.macro_namespaces.push(namespace.clone());
                namespace.clone()
            }
        };

        // We get our macro definition using the namespace name we just got
        let macro_definition = self.macros
            .last()
            .and_then(|m| m.get(&active_namespace))
            .and_then(|m| m.get(macro_name));

        if let Some(&Macro { ref body, ref params, ..}) = macro_definition {
            // fail fast if the number of args don't match
            if params.len() != call_params.len() {
                let params_seen = call_params.keys().cloned().collect::<Vec<String>>();
                bail!("Macro `{}` got `{:?}` for args but was expecting `{:?}` (order does not matter)", macro_name, params, params_seen);
            }
            // We need to make a new context for the macro from the arguments given
            // Return an error if we get some unknown params
            let mut context = Map::new();

            // We don't want to escape variables used as params
            let should_escape = self.should_escape;
            self.should_escape = false;
            for (param_name, exp) in call_params {
                if !params.contains(param_name) {
                    let params_seen = call_params.keys().cloned().collect::<Vec<String>>();
                    bail!("Macro `{}` got `{:?}` for args but was expecting `{:?}` (order does not matter)", macro_name, params, params_seen);
                }
                context.insert(param_name.to_string(), self.eval_expression(exp)?);
            }
            self.should_escape = should_escape;

            // Push this context to our stack of macro context so the renderer can pick variables
            // from it
            self.macro_context.push((context.into(), vec![]));
            // We render the macro body as a normal node
            let mut output = String::new();
            for node in body.get_children() {
                output.push_str(&self.render_node(node)?);
            }
            // If the current namespace wasn't `self`, we remove it since it's not needed anymore
            // In the `self` case, we are still in the parent macro and its namespace is still
            // needed so we keep it
            if namespace == &active_namespace {
                self.macro_namespaces.pop();
            }
            // We remove the macro context we just rendered from our stack of contexts
            self.macro_context.pop();
            Ok(output.trim().to_string())
        } else {
            bail!("Macro `{}` was not found in the namespace `{}`", macro_name, active_namespace);
        }
    }

    /// Renders a given block by going through all the parents
    /// `level` representing how deep we go: 0 is the current template being rendered
    /// and 1 would be its direct parent and so on
    /// If we can't find any other block definitions, the top parent block will be rendered
    fn render_block(&mut self, name: &str, body: &Node, level: usize) -> Result<String> {
        // either we are at the current template or there are no parents left
        let blocks_definitions = if level == 0 || level + 1 > self.template.parents.len() {
            &self.template.blocks_definitions
        } else {
            // there's at least one more parent before the top one
            // level - 1 as 0 is the current template and doesn't count
            &self.tera.get_template(
                &self.template.parents[level - 1]
            ).unwrap().blocks_definitions
        };

        // We pick the first block, ie the one in the template we are rendering
        // We will go up in "level" if we encounter a super()
        match blocks_definitions.get(name) {
            Some(b) => {
                // the indexing here is safe since we are rendering a block, we know we have
                // at least 1
                match &b[0] {
                    &(ref tpl_name, Block { ref body, ..}) => {
                        self.blocks.push((name.to_string(), 0));
                        let has_macro = self.import_macros(tpl_name)?;
                        let res = self.render_node(body);
                        if has_macro {
                            self.macros.pop();
                        }
                        res
                    },
                    x => unreachable!("render_node Block {:?}", x)
                }
            },
            None => {
                if self.template.parents.len() >= level + 1 {
                    self.render_block(name, body, level + 1)
                } else {
                    self.render_node(body)
                }
            }
        }
    }

    fn import_macros(&mut self, tpl_name: &str) -> Result<bool> {
        let tpl = self.tera.get_template(tpl_name)?;
        if tpl.imported_macro_files.is_empty() {
            return Ok(false);
        }

        let mut map = HashMap::new();
        for &(ref filename, ref namespace) in &tpl.imported_macro_files {
            let macro_tpl = self.tera.get_template(filename)?;
            map.insert(namespace.to_string(), &macro_tpl.macros);
        }
        self.macros.push(map);
        Ok(true)
    }

    pub fn render_node(&mut self, node: &Node) -> Result<String> {
        match node {
            &Include(ref p) => {
                let ast = self.tera.get_template(p)?.ast.get_children();
                let mut output = String::new();
                for node in ast {
                    output.push_str(&self.render_node(node)?);
                }

                Ok(output.trim().to_string())
            },
            &ImportMacro {ref tpl_name, ref name} => {
                let tpl = self.tera.get_template(tpl_name)?;
                let mut map = if self.macros.is_empty() {
                    HashMap::new()
                } else {
                    self.macros.pop().unwrap()
                };
                map.insert(name.to_string(), &tpl.macros);
                self.macros.push(map);
                // In theory, the render_node should return Result<Option<String>>
                // but in practice there's no difference so keeping this hack
                Ok("".to_string())
            },
            &MacroCall {..} => self.render_macro(node),
            &Text(ref s) => Ok(s.to_string()),
            &Raw(ref s) => Ok(s.trim().to_string()),
            &FilterSection {ref name, ref params, ref body} => {
                let filter_fn = self.tera.get_filter(name)?;
                let mut all_args = HashMap::new();
                for (arg_name, exp) in params {
                    all_args.insert(arg_name.to_string(), self.eval_expression(exp)?);
                }
                let value = self.render_node(body)?;
                match filter_fn(Value::String(value), all_args)? {
                    Value::String(s) => Ok(s),
                    val => Ok(val.render())
                }
            },
            &VariableBlock(ref exp) => self.render_variable_block(exp),
            &If {ref condition_nodes, ref else_node} => {
                self.render_if(condition_nodes, else_node)
            },
            &List(ref body) => {
                let mut output = String::new();
                for n in body {
                    output.push_str(&self.render_node(n)?);
                }
                Ok(output)
            },
            &For {ref key, ref value, ref container, ref body} => {
                self.render_for(key, value, container, body)
            },
            &Block {ref name, ref body} => self.render_block(name, body, 0),
            &Super => {
                if let Some((name, level)) = self.blocks.pop() {
                    let new_level = level + 1;

                    match self.template.blocks_definitions.get(&name) {
                        Some(b) => {
                            match &b[new_level] {
                                &(ref tpl_name, Block { ref body, .. }) => {
                                    self.blocks.push((name, new_level));
                                    let has_macro = self.import_macros(tpl_name)?;
                                    let res = self.render_node(body);
                                    if has_macro {
                                        self.macros.pop();
                                    }
                                    // Can't go any higher for that block anymore?
                                    if new_level == b.len() - 1 {
                                        // then remove it from the stack, we're done with it
                                        self.blocks.pop();
                                    }
                                    res
                                },
                                x => unreachable!("render_node Block {:?}", x)
                            }
                        },
                        None => unreachable!("render_node -> didn't get block")
                    }
                } else {
                    // prevented by parser already, unless it's a super in the base template
                    // TODO: add a test and see if we need to return an error instead
                    unreachable!("Super called outside of a block or in base template")
                }
            },
            &Set { .. } => self.eval_set(node).and(Ok("".to_string())),
            &GlobalFunctionCall { .. } => Ok(self.eval_global_fn(node)?.render()),
            &Extends(_) | &Macro {..} => Ok("".to_string()),
            x => unreachable!("render_node -> unexpected node: {:?}", x)
        }
    }

    // Helper fn that tries to find the current context: are we in a macro? in a parent template?
    // in order to give the best possible error when getting an error when rendering a tpl
    fn get_error_location(&self) -> String {
        let mut error_location = format!("Failed to render '{}'", self.template.name);

        // in a macro?
        if let Some(macro_namespace) = self.macro_namespaces.last() {
            error_location += &format!(": error while rendering a macro from the `{}` namespace", macro_namespace);
        }

        // which template are we in?
        if let Some(&(ref name, ref level)) = self.blocks.last() {
            let block_def = self.template.blocks_definitions
                .get(name)
                .and_then(|b| b.get(*level));

            if let Some(&(ref tpl_name, _)) = block_def {
                if tpl_name != &self.template.name {
                    error_location += &format!(" (error happened in '{}').", tpl_name);
                }
            }
        } else if let Some(parent) = self.template.parents.last() {
            // Error happened in the base template, outside of blocks
            error_location += &format!(" (error happened in '{}').", parent);
        }

        error_location
    }

    pub fn render(&mut self) -> Result<String> {
        let ast = if !self.template.parents.is_empty() {
            let parent = self.tera.get_template(
                self.template.parents.last().expect("Couldn't get first ancestor template")
            ).chain_err(|| format!("Failed to render '{}'", self.template.name))?;
            parent.ast.get_children()
        } else {
            self.template.ast.get_children()
        };

        let mut output = String::new();
        for node in ast {
            output.push_str(
                &self.render_node(node).chain_err(|| self.get_error_location())?
            );
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use context::Context;
    use errors::Result;
    use tera::Tera;

    fn render_template(content: &str, context: Context) -> Result<String> {
        let mut tera = Tera::default();
        tera.add_raw_template("hello", content).unwrap();

        tera.render("hello", &context)
    }

    #[test]
    fn test_render_include() {
        let mut tera = Tera::default();
        tera.add_raw_template("world", "world").unwrap();
        tera.add_raw_template("hello", "<h1>Hello {% include \"world\" %}</h1>").unwrap();
        let result = tera.render("hello", &Context::new());
        assert_eq!(result.unwrap(), "<h1>Hello world</h1>".to_owned());
    }

    #[test]
    fn test_render_simple_string() {
        let result = render_template("<h1>Hello world</h1>", Context::new());
        assert_eq!(result.unwrap(), "<h1>Hello world</h1>".to_owned());
    }

    #[test]
    fn test_render_math() {
        let tests = vec![
            ("{{ 1 + 1 }}", "2".to_string()),
            ("{{ 1 + 1.1 }}", "2.1".to_string()),
            ("{{ 3 - 1 }}", "2".to_string()),
            ("{{ 3 - 1.1 }}", "1.9".to_string()),
            ("{{ 2 * 5 }}", "10".to_string()),
            ("{{ 10 / 5 }}", "2".to_string()),
            ("{{ 2.1 * 5 }}", "10.5".to_string()),
            ("{{ 2.1 * 5.05 }}", "10.605".to_string()),
            ("{{ 2 / 0.5 }}", "4".to_string()),
            ("{{ 2.1 / 0.5 }}", "4.2".to_string()),
        ];

        for (input, expected) in tests {
            assert_eq!(render_template(input, Context::new()).unwrap(), expected);
        }
    }

    #[test]
    fn test_render_basic_variable() {
        let mut context = Context::new();
        context.add("name", &"Vincent");
        let result = render_template("My name is {{ name }}.", context);
        assert_eq!(result.unwrap(), "My name is Vincent.".to_owned());
    }

    #[test]
    fn test_render_math_with_variable() {
        let mut context = Context::new();
        context.add("vat_rate", &0.20);
        let result = render_template("Vat: £{{ 100 * vat_rate }}.", context);

        assert_eq!(result.unwrap(), "Vat: £20.".to_owned());
    }

    #[test]
    fn test_render_comment() {
        let result = render_template("<h1>Hello {# comment #} world</h1>", Context::new());
        assert_eq!(result.unwrap(), "<h1>Hello  world</h1>".to_owned());
    }

    #[test]
    fn test_render_nested_comment() {
        let result = render_template("<h1>Hello {# comment {# nested #} world</h1>", Context::new());
        assert_eq!(result.unwrap(), "<h1>Hello  world</h1>".to_owned());
    }

    #[test]
    fn test_ignore_variable_in_comment() {
        let mut context = Context::new();
        context.add("name", &"Vincent");
        let result = render_template("My name {# was {{ name }} #} is No One.", context);

        assert_eq!(result.unwrap(), "My name  is No One.".to_owned());
    }

    #[test]
    fn test_render_if_simple() {
        let mut context = Context::new();
        context.add("is_admin", &true);
        let result = render_template("{% if is_admin %}Admin{% endif %}", context);

        assert_eq!(result.unwrap(), "Admin".to_owned());
    }

    #[test]
    fn test_render_if_or_conditions() {
        let mut context = Context::new();
        context.add("is_adult", &false);
        context.add("age", &18);
        let result = render_template("{% if is_adult or age + 1 > 18 %}Adult{% endif %}", context);

        assert_eq!(result.unwrap(), "Adult".to_owned());
    }

    #[test]
    fn test_render_if_and_conditions_with_equality() {
        let mut context = Context::new();
        context.add("is_adult", &true);
        context.add("age", &18);
        let result = render_template("{% if is_adult and age == 18 %}Adult{% endif %}", context);

        assert_eq!(result.unwrap(), "Adult".to_owned());
    }

    #[test]
    fn test_render_basic_for() {
        let mut context = Context::new();
        context.add("data", &vec![1,2,3]);
        let result = render_template("{% for i in data %}{{i}}{% endfor %}", context);

        assert_eq!(result.unwrap(), "123".to_owned());
    }


    #[test]
    fn test_render_key_value_for() {
        let mut context = Context::new();
        let mut map = BTreeMap::new();
        map.insert("name", "bob");
        map.insert("age", "18");
        context.add("data", &map);
        let result = render_template("{% for key, val in data %}{{key}}:{{val}} {% endfor %}", context);

        assert_eq!(result.unwrap(), "age:18 name:bob".to_owned());
    }
    #[test]
    fn test_render_loop_variables() {
        let mut context = Context::new();
        context.add("data", &vec![1,2,3]);
        let result = render_template(
            "{% for i in data %}{{loop.index}}{{loop.index0}}{{loop.first}}{{loop.last}}{% endfor %}",
            context
        );

        assert_eq!(result.unwrap(), "10truefalse21falsefalse32falsetrue".to_owned());
    }

    #[test]
    fn test_render_nested_loop_simple() {
        let mut context = Context::new();
        context.add("vectors", &vec![vec![0, 3, 6], vec![1, 4, 7]]);
        let result = render_template(
            "{% for vector in vectors %}{% for j in vector %}{{ j }}{% endfor %}{% endfor %}",
            context
        );

        assert_eq!(result.unwrap(), "036147".to_owned());
    }

    #[test]
    fn test_render_nested_loop_with_empty_vec() {
        let mut context = Context::new();
        context.add("vectors", &vec![vec![0, 3, 6], vec![], vec![1, 4, 7]]);
        let result = render_template(
            "{% for vector in vectors %}{% for j in vector %}{{ j }}{% endfor %}{% endfor %}",
            context
        );

        assert_eq!(result.unwrap(), "036147".to_owned());
    }

    #[test]
    fn test_render_filter() {
        let mut context = Context::new();
        context.add("greeting", &"hello");
        let result = render_template(
            "{{ greeting | upper }}",
            context
        );

        assert_eq!(result.unwrap(), "HELLO".to_owned());
    }

    #[test]
    fn test_render_filter_section() {
        let context = Context::new();
        let result = render_template(
            "{% filter upper %}Hello{% endfilter %}",
            context
        );

        assert_eq!(result.unwrap(), "HELLO".to_owned());
    }

    #[test]
    fn test_render_string_in_variable_braces() {
        let context = Context::new();
        let result = render_template(r#"{{ "{{ hey }}" }}"#, context);
        assert_eq!(result.unwrap(), "{{ hey }}".to_owned());
    }

    #[test]
    fn test_render_index_array() {
        let mut context = Context::new();
        context.add("my_arr", &vec![1, 2, 3]);
        context.add("my_arr2", &vec![(1,2,3), (1,2,3), (1,2,3)]);
        let result = render_template(
            "{{ my_arr.1 }}{{ my_arr2.1.1 }}",
            context
        );

        assert_eq!(result.unwrap(), "22".to_owned());
    }

    // this was a regression in 0.3.0
    #[test]
    fn test_render_if_in_for() {
        let mut context = Context::new();
        context.add("sel", &2u32);
        context.add("seq", &vec![1,2,3]);
        let result = render_template(
            "{% for val in seq %} {% if val == sel %} on {% else %} off {% endif %} {% endfor %}",
            context
        );
        assert_eq!(result.unwrap(), "off on off".to_string());
    }

    #[test]
    fn test_autoescape_html() {
        let mut context = Context::new();
        context.add("bad", &"<script>alert('pwnd');</script>");
        let mut tera = Tera::default();
        tera.add_raw_template("hello.html", "{{bad}}").unwrap();
        let result = tera.render("hello.html", &context);

        assert_eq!(result.unwrap(), "&lt;script&gt;alert(&#x27;pwnd&#x27;);&lt;&#x2F;script&gt;".to_string());
    }

    #[test]
    fn test_no_autoescape_on_extensions_not_specified() {
        let mut context = Context::new();
        context.add("bad", &"<script>alert('pwnd');</script>");
        let mut tera = Tera::default();
        tera.add_raw_template("hello.sql", "{{bad}}").unwrap();
        let result = tera.render("hello.sql", &context);

        assert_eq!(result.unwrap(), "<script>alert('pwnd');</script>".to_string());
    }

    #[test]
    fn test_no_autoescape_with_safe_filter() {
        let mut context = Context::new();
        context.add("bad", &"<script>alert('pwnd');</script>");
        let mut tera = Tera::default();
        tera.add_raw_template("hello.html", "{{ bad | safe }}").unwrap();
        let result = tera.render("hello.html", &context);

        assert_eq!(result.unwrap(), "<script>alert('pwnd');</script>".to_string());
    }

    #[test]
    fn test_render_super_multiple_inheritance() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("grandparent", "{% block hey %}hello{% endblock hey %} {% block ending %}sincerely{% endblock ending %}"),
            ("parent", "{% extends \"grandparent\" %}{% block hey %}hi and grandma says {{ super() }}{% endblock hey %}"),
            ("child", "{% extends \"parent\" %}{% block hey %}dad says {{ super() }}{% endblock hey %}{% block ending %}{{ super() }} with love{% endblock ending %}"),
        ]).unwrap();
        let result = tera.render("child", &Context::new());

        assert_eq!(result.unwrap(), "dad says hi and grandma says hello sincerely with love".to_string());
    }

    #[test]
    fn test_render_super_multiple_inheritance_nested_block() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("grandparent", "{% block hey %}hello{% endblock hey %}"),
            ("parent", "{% extends \"grandparent\" %}{% block hey %}hi and grandma says {{ super() }} {% block ending %}sincerely{% endblock ending %}{% endblock hey %}"),
            ("child", "{% extends \"parent\" %}{% block hey %}dad says {{ super() }}{% endblock hey %}{% block ending %}{{ super() }} with love{% endblock ending %}"),
        ]).unwrap();
        let result = tera.render("child", &Context::new());

        assert_eq!(result.unwrap(), "dad says hi and grandma says hello sincerely with love".to_string());
    }

    #[test]
    fn test_render_nested_block_multiple_inheritance_no_super() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("index", "{% block content%}INDEX{% endblock content %}"),
            ("docs", "{% extends \"index\" %}{% block content%}DOCS{% block more %}MORE{% endblock more %}{% endblock content %}"),
            ("page", "{% extends \"docs\" %}{% block more %}PAGE{% endblock more %}"),
        ]).unwrap();

        let result = tera.render("page", &Context::new());

        assert_eq!(result.unwrap(), "DOCSPAGE".to_string());
    }

    #[test]
    fn test_render_multiple_inheritance_no_super() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("top", "{% block pre %}{% endblock pre %}{% block main %}{% endblock main %}"),
            ("mid", "{% extends \"top\" %}{% block pre %}PRE{% endblock pre %}"),
            ("bottom", "{% extends \"mid\" %}{% block main %}MAIN{% endblock main %}"),
        ]).unwrap();

        let result = tera.render("bottom", &Context::new());

        assert_eq!(result.unwrap(), "PREMAIN".to_string());
    }


    #[test]
    fn test_render_macros() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("macros", "{% macro hello()%}Hello{% endmacro hello %}"),
            ("tpl", "{% import \"macros\" as macros %}{% block hey %}{{macros::hello()}}{% endblock hey %}"),
        ]).unwrap();

        let result = tera.render("tpl", &Context::new());

        assert_eq!(result.unwrap(), "Hello".to_string());
    }

    #[test]
    fn test_render_macros_expression_arg() {
        let mut context = Context::new();
        context.add("pages", &vec![1,2,3,4,5]);
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("macros", "{% macro hello(val)%}{{val}}{% endmacro hello %}"),
            ("tpl", "{% import \"macros\" as macros %}{{macros::hello(val=pages|last)}}"),
        ]).unwrap();

        let result = tera.render("tpl", &context);

        assert_eq!(result.unwrap(), "5".to_string());
    }

    #[test]
    fn test_render_macros_in_child_templates_same_namespace() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("grandparent", "{% block hey %}hello{% endblock hey %}"),
            ("macros", "{% macro hello()%}Hello{% endmacro hello %}"),
            ("macros2", "{% macro hi()%}Hi{% endmacro hi %}"),
            ("parent", "{% extends \"grandparent\" %}{% import \"macros\" as macros %}{% block hey %}{{macros::hello()}}{% endblock hey %}"),
            ("child", "{% extends \"parent\" %}{% import \"macros2\" as macros %}{% block hey %}{{super()}}/{{macros::hi()}}{% endblock hey %}"),
        ]).unwrap();

        let result = tera.render("child", &Context::new());

        assert_eq!(result.unwrap(), "Hello/Hi".to_string());
    }

    #[test]
    fn test_render_macros_in_child_templates_different_namespace() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("grandparent", "{% block hey %}hello{% endblock hey %}"),
            ("macros", "{% macro hello()%}Hello{% endmacro hello %}"),
            ("macros2", "{% macro hi()%}Hi{% endmacro hi %}"),
            ("parent", "{% extends \"grandparent\" %}{% import \"macros\" as macros %}{% block hey %}{{macros::hello()}}{% endblock hey %}"),
            ("child", "{% extends \"parent\" %}{% import \"macros2\" as macros2 %}{% block hey %}{{super()}}/{{macros2::hi()}}{% endblock hey %}"),
        ]).unwrap();

        let result = tera.render("child", &Context::new());

        assert_eq!(result.unwrap(), "Hello/Hi".to_string());
    }

    #[test]
    fn test_render_macros_in_parent_template_with_inheritance() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("macros", "{% macro hello()%}Hello{% endmacro hello %}"),
            ("grandparent", "{% import \"macros\" as macros %}{% block hey %}{{macros::hello()}}{% endblock hey %}"),
            ("child", "{% extends \"grandparent\" %}{% import \"macros\" as macros %}{% block hey %}{{super()}}/{{macros::hello()}}{% endblock hey %}"),
        ]).unwrap();

        let result = tera.render("child", &Context::new());

        assert_eq!(result.unwrap(), "Hello/Hello".to_string());
    }

    #[test]
    fn test_render_not_condition_simple_value_exists() {
        let mut context = Context::new();
        context.add("logged_in", &false);
        let mut tera = Tera::default();
        tera.add_raw_template("hello.html", "{% if not logged_in %}Login{% endif %}").unwrap();
        let result = tera.render("hello.html", &context);

        assert_eq!(result.unwrap(), "Login".to_string());
    }

    #[test]
    fn test_render_not_condition_simple_value_does_not_exist() {
        let mut tera = Tera::default();
        tera.add_raw_template("hello.html", "{% if not logged_in %}Login{% endif %}").unwrap();
        let result = tera.render("hello.html", &Context::new());

        assert_eq!(result.unwrap(), "Login".to_string());
    }

    #[test]
    fn test_render_not_complex_condition_and() {
        let mut context = Context::new();
        context.add("logged_in", &false);
        context.add("active", &true);
        let mut tera = Tera::default();
        tera.add_raw_template("hello.html", "{% if not logged_in and active %}Login{% endif %}").unwrap();
        let result = tera.render("hello.html", &context);

        assert_eq!(result.unwrap(), "Login".to_string());
    }

    #[test]
    fn test_render_not_complex_condition_or() {
        let mut context = Context::new();
        context.add("number_users", &11);
        context.add("active", &true);
        let mut tera = Tera::default();
        tera.add_raw_template("hello.html", "{% if not active or number_users > 10 %}Login{% endif %}").unwrap();
        let result = tera.render("hello.html", &context);

        assert_eq!(result.unwrap(), "Login".to_string());
    }

    #[test]
    fn test_render_global_fn_in_variable_block() {
        let mut tera = Tera::default();
        tera.add_raw_template("hello.html", "{{ range(end=5) }}").unwrap();
        let result = tera.render("hello.html", &Context::new());

        assert_eq!(result.unwrap(), "[0, 1, 2, 3, 4, ]".to_string());
    }

    #[test]
    fn test_render_global_fn_for_container() {
        let mut tera = Tera::default();
        tera.add_raw_template("hello.html", "{% for i in range(end=5) %}{{i}}{% endfor %}").unwrap();
        let result = tera.render("hello.html", &Context::new());

        assert_eq!(result.unwrap(), "01234".to_string());
    }

    #[test]
    fn test_render_set_tag_literal() {
        let mut tera = Tera::default();
        tera.add_raw_template("hello.html", "{% set my_var = \"hello\" %}{{my_var}}").unwrap();
        let result = tera.render("hello.html", &Context::new());

        assert_eq!(result.unwrap(), "hello".to_string());
    }

    #[test]
    fn test_render_set_tag_expression() {
        let mut tera = Tera::default();
        tera.add_raw_template("hello.html", "{% set my_var = 1 + 5 %}{{my_var}}").unwrap();
        let result = tera.render("hello.html", &Context::new());

        assert_eq!(result.unwrap(), "6".to_string());
    }

    #[test]
    fn test_render_set_tag_forloop_scope() {
        let mut tera = Tera::default();
        tera.add_raw_template(
            "hello.html",
            "{% set looped = 0 %}{% for i in range(end=5) %}{% set looped = i %}{{looped}}{% endfor%}{{looped}}"
        ).unwrap();
        let result = tera.render("hello.html", &Context::new());

        assert_eq!(result.unwrap(), "012340".to_string());
    }

    #[test]
    fn test_render_set_tag_variable() {
        let mut tera = Tera::default();
        tera.add_raw_template("hello.html", "{% set my_var = hello %}{{my_var}}").unwrap();
        let mut context = Context::new();
        context.add("hello", &5);
        let result = tera.render("hello.html", &context);

        assert_eq!(result.unwrap(), "5".to_string());
    }

    #[test]
    fn test_render_set_tag_global_fn() {
        let mut tera = Tera::default();
        tera.add_raw_template("hello.html", "{% set my_var = range(end=5) %}{{my_var}}").unwrap();
        let result = tera.render("hello.html", &Context::new());

        assert_eq!(result.unwrap(), "[0, 1, 2, 3, 4, ]".to_string());
    }

    #[test]
    fn test_render_set_tag_macro() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("macros", "{% macro hello()%}Hello{% endmacro hello %}"),
            ("hello.html", "{% import \"macros\" as macros %}{% set my_var = macros::hello() %}{{my_var}}"),
        ]).unwrap();
        let result = tera.render("hello.html", &Context::new());

        assert_eq!(result.unwrap(), "Hello".to_string());
    }

    #[test]
    fn test_error_location_basic() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("tpl", "{{ 1 + true }}"),
        ]).unwrap();

        let result = tera.render("tpl", &Context::new());

        assert_eq!(
            result.unwrap_err().iter().nth(0).unwrap().description(),
            "Failed to render \'tpl\'"
        );
    }

    #[test]
    fn test_error_location_inside_macro() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("macros", "{% macro hello()%}{{ 1 + true }}{% endmacro hello %}"),
            ("tpl", "{% import \"macros\" as macros %}{{ macro::hello() }}"),
        ]).unwrap();

        let result = tera.render("tpl", &Context::new());

        assert_eq!(
            result.unwrap_err().iter().nth(0).unwrap().description(),
            "Failed to render \'tpl\': error while rendering a macro from the `macro` namespace"
        );
    }

    #[test]
    fn test_error_location_base_template() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("parent", "Hello {{ greeting + 1}} {% block bob %}{% endblock bob %}"),
            ("child", "{% extends \"parent\" %}{% block bob %}Hey{% endblock bob %}"),
        ]).unwrap();

        let result = tera.render("child", &Context::new());

        assert_eq!(
            result.unwrap_err().iter().nth(0).unwrap().description(),
            "Failed to render \'child\' (error happened in 'parent')."
        );
    }

    #[test]
    fn test_error_location_in_parent_block() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("parent", "Hello {{ greeting }} {% block bob %}{{ 1 + true }}{% endblock bob %}"),
            ("child", "{% extends \"parent\" %}{% block bob %}{{ super() }}Hey{% endblock bob %}"),
        ]).unwrap();

        let result = tera.render("child", &Context::new());

        assert_eq!(
            result.unwrap_err().iter().nth(0).unwrap().description(),
            "Failed to render \'child\' (error happened in 'parent')."
        );
    }

    #[test]
    fn test_error_location_in_parent_in_macro() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("macros", "{% macro hello()%}{{ 1 + true }}{% endmacro hello %}"),
            ("parent", "{% import \"macros\" as macros %}{{ macro::hello() }}{% block bob %}{% endblock bob %}"),
            ("child", "{% extends \"parent\" %}{% block bob %}{{ super() }}Hey{% endblock bob %}"),
        ]).unwrap();

        let result = tera.render("child", &Context::new());

        assert_eq!(
            result.unwrap_err().iter().nth(0).unwrap().description(),
            "Failed to render \'child\': error while rendering a macro from the `macro` namespace (error happened in \'parent\')."
        );
    }

    // https://github.com/Keats/tera/issues/184
    #[test]
    fn can_use_note_as_for_variable() {
        let mut tera = Tera::default();
        tera.add_raw_template("hello.html", "{% for note in notes %}{{ note }}{% endfor %}").unwrap();
        let mut context = Context::new();
        context.add("notes", &vec![1,2,4]);
        let result = tera.render("hello.html", &context);

        assert_eq!(result.unwrap(), "124".to_string());
    }

    // https://github.com/Keats/tera/issues/187
    #[test]
    fn can_use_operators_in_if_condition() {
        let mut tera = Tera::default();
        tera.add_raw_template("lte", "{% if 1 <= 2 %}a{% endif %}").unwrap();
        tera.add_raw_template("lt", "{% if 1 < 2 %}a{% endif %}").unwrap();
        tera.add_raw_template("gte", "{% if 2 >= 1 %}a{% endif %}").unwrap();
        tera.add_raw_template("gt", "{% if 2 > 1 %}a{% endif %}").unwrap();
        tera.add_raw_template("eq", "{% if 1 == 1 %}a{% endif %}").unwrap();
        tera.add_raw_template("neq", "{% if 2 != 1 %}a{% endif %}").unwrap();

        assert_eq!(tera.render("lte", &Context::new()).unwrap(), "a".to_string());
        assert_eq!(tera.render("lt", &Context::new()).unwrap(), "a".to_string());
        assert_eq!(tera.render("gte", &Context::new()).unwrap(), "a".to_string());
        assert_eq!(tera.render("gt", &Context::new()).unwrap(), "a".to_string());
        assert_eq!(tera.render("eq", &Context::new()).unwrap(), "a".to_string());
        assert_eq!(tera.render("neq", &Context::new()).unwrap(), "a".to_string());
    }

    // https://github.com/Keats/tera/issues/188
    #[test]
    fn doesnt_fallthrough_elif() {
        let mut tera = Tera::default();
        tera.add_raw_template("ifs", "{% if 1 < 4 %}a{% elif 2 < 4 %}b{% elif 3 < 4 %}c{% else %}d{% endif %}").unwrap();
        assert_eq!(tera.render("ifs", &Context::new()).unwrap(), "a".to_string());
    }

    // https://github.com/Keats/tera/issues/189
    #[test]
    fn doesnt_panic_with_nan_results() {
        let mut tera = Tera::default();
        tera.add_raw_template("render", "{{ 0 / 0 }}").unwrap();
        tera.add_raw_template("set", "{% set x = 0 / 0 %}{{ x }}").unwrap();
        tera.add_raw_template("condition", "{% if 0 / 0 %}a{% endif %}").unwrap();

        assert_eq!(tera.render("render", &Context::new()).unwrap(), "NaN".to_string());
        assert_eq!(tera.render("set", &Context::new()).unwrap(), "NaN".to_string());
        assert_eq!(tera.render("condition", &Context::new()).unwrap(), "".to_string());
    }

    #[test]
    fn test_set_tag_variable_doesnt_escape_it() {
        let mut tera = Tera::default();
        tera.add_raw_template("hello.html", "{% set my_var = hello %}{{my_var | safe}}").unwrap();
        let mut context = Context::new();
        context.add("hello", &"&");
        let result = tera.render("hello.html", &context);

        assert_eq!(result.unwrap(), "&".to_string());
    }

    #[test]
    fn test_filter_param_doesnt_escape_it() {
        let mut tera = Tera::default();
        tera.add_raw_template("hello.html", r#"{{ my_var | replace(from="h", to=to) | safe}}"#).unwrap();
        let mut context = Context::new();
        context.add("my_var", &"hey");
        context.add("to", &"&");
        let result = tera.render("hello.html", &context);

        assert_eq!(result.unwrap(), "&ey".to_string());
    }

    #[test]
    fn test_macro_param_doesnt_escape_it() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("macros.html", r#"{% macro print(val) %}{{val|safe}}{% endmacro print %}"#),
            ("hello.html", r#"{% import "macros.html" as macros %}{{ macros::print(val=my_var)}}"#),
        ]).unwrap();
        let mut context = Context::new();
        context.add("my_var", &"&");
        let result = tera.render("hello.html", &context);

        assert_eq!(result.unwrap(), "&".to_string());
    }
}
