mod forloop;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use serde_json::to_string_pretty;
use serde_json::value::{Value, to_value, Number};
use serde_json::map::{Map as JsonMap};

use self::forloop::{ForLoop};
use parser::ast::*;
use template::Template;
use tera::Tera;
use errors::{Result, ResultExt};
use context::{ValueRender, ValueNumber, ValueTruthy, get_json_pointer};
use utils::escape_html;


static MAGICAL_DUMP_VAR: &'static str = "__tera_context";


#[derive(Debug)]
pub struct Renderer<'a> {
    template: &'a Template,
    context: Value,
    tera: &'a Tera,
    /// All ongoing forloops
    for_loops: Vec<ForLoop>,
    /// Looks like Vec<filename: {macro_name: macro_def}>
    macros: Vec<HashMap<String, &'a HashMap<String, MacroDefinition>>>,
    /// Set when rendering macros, empty if not in a macro. Loops in macros are set
    /// as part of the macro_context and not for_loops
    /// Vec<(MacroContext, Vec<loops>)>
    macro_context: Vec<(Value, Vec<ForLoop>)>,
    /// Keeps track of which macro namespace we're on in order to resolve the `self::` syntax
    macro_namespaces: Vec<String>,
    /// Whether this template should be escaped or not
    should_escape: bool,
    /// Used when super() is used in a block, to know where we are in our stack of
    /// definitions and for which block
    /// Vec<(block name, level)>
    blocks: Vec<(String, usize)>,
}


impl<'a> Renderer<'a> {
    pub fn new(template: &'a Template, tera: &'a Tera, context: Value) -> Renderer<'a> {
        let should_escape = tera.autoescape_suffixes.iter().any(|ext| {
            // We prefer a `path` if set, otherwise use the `name`
            if let Some(ref p) = template.path {
                return p.ends_with(ext);
            }
            template.name.ends_with(ext)
        });

        Renderer {
            template,
            tera,
            context,
            should_escape,
            for_loops: vec![],
            macros: vec![],
            macro_context: vec![],
            macro_namespaces: vec![],
            blocks: vec![],
        }
    }

    /// Lookup a key in the context, taking into account macros and loops
    fn lookup_ident(&mut self, key: &str) -> Result<Value> {
        // Differentiate between macros and general context
        let (context, for_loops) = match self.macro_context.last() {
            Some(c) => (&c.0, &c.1),
            None => (&self.context, &self.for_loops)
        };

        // Magical variable that just dumps the context
        if key == MAGICAL_DUMP_VAR {
            // Unwraps are safe since we are dealing with things that are already Value
            return Ok(to_value(to_string_pretty(context).unwrap()).unwrap());
        }

        #[inline]
        fn find_variable(context: &Value, key: &str, tpl_name: &str) -> Result<Value> {
            match context.pointer(&get_json_pointer(key)) {
                Some(v) => Ok(v.clone()),
                None => bail!("Variable `{}` not found in context while rendering '{}'", key, tpl_name)
            }
        }

        // Look in the plain context if we aren't in a for loop
        if for_loops.is_empty() {
            return find_variable(context, key, &self.template.name);
        }

        // Separates the initial key (anything before a dot) from everything after
        let (real_key, tail) = if let Some(tail_pos) = key.find('.') {
            (&key[..tail_pos], &key[tail_pos + 1..])
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

            if let Some(v) = value {
                if tail.is_empty() {
                    return Ok(v.clone());
                }
                // A struct or some nested structure
                return find_variable(v, tail, &self.template.name)
                    .chain_err(|| format!("Variable lookup failed in forloop for `{}`", key));
            }
        }

        // Gets there when looking a variable in the global context while in a forloop
        find_variable(context, key, &self.template.name)
    }

    /// Inserts the result of the expression in the right context with `key` as the ... key
    fn eval_set(&mut self, set: &Set) -> Result<()> {
        let val = self.safe_eval_expression(&set.value)?;

        let context = match self.macro_context.last_mut() {
            Some(c) => c.0.as_object_mut().unwrap(),
            None => match self.for_loops.last_mut() {
                Some(f) => if set.global { self.context.as_object_mut().unwrap() } else { &mut f.extra_values },
                None => self.context.as_object_mut().unwrap()
            },
        };

        context.insert(set.key.to_string(), val);
        Ok(())
    }

    /// In some cases, we will have filters in lhs/rhs of a math expression
    /// `eval_as_number` only works on ExprVal rather than Expr
    fn eval_expr_as_number(&mut self, expr: &Expr) -> Result<f64> {
        if !expr.filters.is_empty() {
            match self.eval_expression(expr)? {
                Value::Number(s) => Ok(s.as_f64().unwrap()),
                _ => bail!("Tried to do math with an expression not resulting in a number"),
            }
        } else {
            self.eval_as_number(&expr.val)
        }
    }

    /// Return the value of an expression as a number
    fn eval_as_number(&mut self, expr: &ExprVal) -> Result<f64> {
        let res = match *expr {
            ExprVal::Ident(ref ident) => {
                match self.lookup_ident(ident)?.as_f64() {
                    Some(v) => v,
                    None => bail!("Variable `{}` was used in a math operation but is not a number", ident)
                }
            },
            ExprVal::Int(val) => val as f64,
            ExprVal::Float(val) => val,
            ExprVal::Math(MathExpr { ref lhs, ref rhs, ref operator }) => {
                let l = self.eval_expr_as_number(&lhs)?;
                let r = self.eval_expr_as_number(&rhs)?;
                match *operator {
                    MathOperator::Mul => l * r,
                    MathOperator::Div => l / r,
                    MathOperator::Add => l + r,
                    MathOperator::Sub => l - r,
                    MathOperator::Modulo => l % r,
                }
            },
            ExprVal::String(ref val) => bail!("Tried to do math with a string: `{}`", val),
            ExprVal::Bool(val) => bail!("Tried to do math with a boolean: `{}`", val),
            _ => unreachable!("unimplemented"),
        };

        Ok(res)
    }

    /// Return the value of an expression as a bool
    fn eval_as_bool(&mut self, expr: &Expr) -> Result<bool> {
        let res = match expr.val {
            ExprVal::Logic(LogicExpr { ref lhs, ref rhs, ref operator}) => {
                match *operator {
                    LogicOperator::Or => self.eval_as_bool(lhs)? || self.eval_as_bool(rhs)?,
                    LogicOperator::And => self.eval_as_bool(lhs)? && self.eval_as_bool(rhs)?,
                    LogicOperator::Gt | LogicOperator::Gte | LogicOperator::Lt | LogicOperator::Lte => {
                        let l = self.eval_expr_as_number(&lhs)?;
                        let r = self.eval_expr_as_number(&rhs)?;

                        match *operator {
                            LogicOperator::Gte => l >= r,
                            LogicOperator::Gt => l > r,
                            LogicOperator::Lte => l <= r,
                            LogicOperator::Lt => l < r,
                            _ => unreachable!()
                        }
                    },
                    LogicOperator::Eq | LogicOperator::NotEq => {
                        let mut lhs_val = self.eval_expression(lhs)?;
                        let mut rhs_val = self.eval_expression(rhs)?;

                        // Monomorphize number vals.
                        if lhs_val.is_number() || rhs_val.is_number() {
                            // We're not implementing JS so can't compare things of different types
                            if !lhs_val.is_number() || !rhs_val.is_number() {
                                return Ok(false);
                            }

                            lhs_val = Value::Number(Number::from_f64(lhs_val.as_f64().unwrap()).unwrap());
                            rhs_val = Value::Number(Number::from_f64(rhs_val.as_f64().unwrap()).unwrap());
                        }

                        match *operator {
                            LogicOperator::Eq => lhs_val == rhs_val,
                            LogicOperator::NotEq => lhs_val != rhs_val,
                            _ => unreachable!()
                        }
                    },
                }
            },
            ExprVal::Ident(ref ident) => self.lookup_ident(ident).map(|v| v.is_truthy()).unwrap_or(false),
            ExprVal::Math(_) | ExprVal::Int(_) | ExprVal::Float(_) => {
                self.eval_as_number(&expr.val).map(|v| v != 0.0 && !v.is_nan())?
            },
            ExprVal::Test(ref test) => self.eval_test(test).unwrap_or(false),
            ExprVal::Bool(val) => val,
            ExprVal::String(ref string) => !string.is_empty(),
            _ => unreachable!("unimplemented logic operation for {:?}", expr),
        };

        if expr.negated {
            return Ok(!res);
        }

        Ok(res)
    }

    fn eval_test(&mut self, test: &Test) -> Result<bool> {
        let tester_fn = self.tera.get_tester(&test.name)?;

        let mut tester_args = vec![];
        for arg in &test.args {
            tester_args.push(self.safe_eval_expression(arg)?);
        }

        Ok(tester_fn(self.lookup_ident(&test.ident).ok(), tester_args)?)
    }

    fn eval_global_fn_call(&mut self, fn_call: &FunctionCall) -> Result<Value> {
        let global_fn = self.tera.get_global_function(&fn_call.name)?;

        let mut args = HashMap::new();
        for (arg_name, expr) in &fn_call.args {
            args.insert(arg_name.to_string(), self.safe_eval_expression(expr)?);
        }

        global_fn(args)
    }

    fn eval_macro_call(&mut self, macro_call: &MacroCall) -> Result<String> {
        // We need to find the active namespace in Tera if `self` is used
        // Since each macro (other than the `self` ones) pushes its own namespace
        // to the stack when being rendered, we can just lookup the last namespace that was pushed
        // to find out the active one
        let active_namespace = match macro_call.namespace.as_ref() {
            "self" => {
                // This happens when calling a macro defined in the file itself without imports
                self.macro_namespaces
                    .last()
                    .expect("Open an issue with a template sample please (mention `self namespace macro`)!")
                    .to_string()
            },
            _ => {
                self.macro_namespaces.push(macro_call.namespace.clone());
                macro_call.namespace.clone()
            }
        };

        // We get our macro definition using the namespace name we just got
        let macro_definition = self.macros
            .last()
            .and_then(|m| m.get(&active_namespace))
            .and_then(|m| m.get(&macro_call.name))
            .ok_or_else(|| format!("Macro `{}` was not found in the namespace `{}`", macro_call.name, active_namespace))?;

        let mut macro_context = JsonMap::new();

        // First the default arguments
        for (arg_name, default_value) in &macro_definition.args {
            let value = match macro_call.args.get(arg_name) {
                Some(val) => self.safe_eval_expression(val)?,
                None => match *default_value {
                    Some(ref val) => self.safe_eval_expression(val)?,
                    None => bail!("Macro `{}` is missing the argument `{}`", macro_call.name, arg_name),
                }
            };
            macro_context.insert(arg_name.to_string(), value);
        }
        // Push this context to our stack of macro context so the renderer can pick variables
        // from it
        self.macro_context.push((macro_context.into(), vec![]));
        let output = self.render_body(&macro_definition.body)?;
        // If the current namespace wasn't `self`, we remove it since it's not needed anymore
        // In the `self` case, we are still in the parent macro and its namespace is still
        // needed so we keep it
        if macro_call.namespace == active_namespace {
            self.macro_namespaces.pop();
        }
        // We remove the macro context we just rendered from our stack of contexts
        self.macro_context.pop();
        Ok(output)
    }

    fn eval_filter(&mut self, value: Value, filter: &FunctionCall) -> Result<Value> {
        let filter_fn = self.tera.get_filter(&filter.name)?;

        let mut args = HashMap::new();
        for (arg_name, expr) in &filter.args {
            args.insert(arg_name.to_string(), self.safe_eval_expression(expr)?);
        }

        filter_fn(value, args)
    }

    fn eval_expression(&mut self, expr: &Expr) -> Result<Value> {
        let mut needs_escape = false;

        let mut res = match expr.val {
            ExprVal::String(ref val) => {
                needs_escape = true;
                Value::String(val.to_string())
            },
            ExprVal::Int(val) => Value::Number(val.into()),
            ExprVal::Float(val) => Value::Number(Number::from_f64(val).unwrap()),
            ExprVal::Bool(val) => Value::Bool(val),
            ExprVal::Ident(ref ident) => {
                needs_escape = ident != MAGICAL_DUMP_VAR;
                // Negated idents are special cased as `not undefined_ident` should not
                // error but instead be falsy values
                match self.lookup_ident(ident) {
                    Ok(val) => val,
                    Err(e) => {
                        if expr.has_default_filter() {
                            if let Some(default_expr) = expr.filters[0].args.get("value") {
                                self.eval_expression(default_expr)?
                            } else {
                                bail!("The `default` filter requires a `value` argument.");
                            }
                        } else {
                            if !expr.negated {
                                return Err(e);
                            }
                            // A negative undefined ident is !false so truthy
                            return Ok(Value::Bool(true));
                        }
                    }
                }
            },
            ExprVal::FunctionCall(ref fn_call) => {
                needs_escape = true;
                self.eval_global_fn_call(fn_call)?
            },
            ExprVal::MacroCall(ref macro_call) => Value::String(self.eval_macro_call(macro_call)?),
            ExprVal::Test(ref test) => Value::Bool(self.eval_test(test)?),
            ExprVal::Logic(_) => Value::Bool(self.eval_as_bool(expr)?),
            ExprVal::Math(_) => {
                let result = self.eval_as_number(&expr.val)?;

                match Number::from_f64(result) {
                    Some(x) => Value::Number(x),
                    None => Value::String("NaN".to_string())
                }
            },
            _ => unreachable!("{:?}", expr),
        };

        // Checks if it's a string and we need to escape it (if the first filter is `safe` we don't)
        if self.should_escape && needs_escape && res.is_string() && expr.filters.first().map_or(true, |f| f.name != "safe") {
            res = to_value(escape_html(res.as_str().unwrap()))?;
        }

        for filter in &expr.filters {
            if filter.name == "safe" || filter.name == "default" {
                continue;
            }
            res = self.eval_filter(res, filter)?;
        }

        // Lastly, we need to check if the expression is negated, thus turning it into a bool
        if expr.negated {
            return Ok(Value::Bool(!res.is_truthy()));
        }

        Ok(res)
    }

    /// A wrapper around `eval_expression` that disables escaping before calling it and sets it back
    /// after. Used when evaluating expressions where we never want escaping, such as function
    /// arguments.
    fn safe_eval_expression(&mut self, expr: &Expr) -> Result<Value> {
        let should_escape = self.should_escape;
        self.should_escape = false;
        let res = self.eval_expression(expr);
        self.should_escape = should_escape;
        res
    }

    fn render_if(&mut self, node: &If) -> Result<String> {
        for &(_, ref expr, ref body) in &node.conditions {
            if self.eval_as_bool(expr)? {
                return self.render_body(body);
            }
        }

        if let Some((_, ref body)) = node.otherwise {
            return self.render_body(body);
        }

        Ok(String::new())
    }

    fn render_for(&mut self, node: &Forloop) -> Result<String> {
        let container_name = match node.container.val {
            ExprVal::Ident(ref ident) => ident,
            ExprVal::FunctionCall(FunctionCall {ref name, ..}) => name,
            _ => bail!("Forloop containers have to be an ident or a function call (tried to iterate on '{:?}')", node.container.val)
        };

        let container_val = self.safe_eval_expression(&node.container)?;

        let for_loop = match container_val {
            Value::Array(_) => {
                if node.key.is_some() {
                    bail!("Tried to iterate using key value on variable `{}`, but it isn't an object/map", container_name);
                }
                ForLoop::new(&node.value, container_val)
            },
            Value::Object(_) => {
                if node.key.is_none() {
                    bail!("Tried to iterate using key value on variable `{}`, but it is missing a key", container_name);
                }
                ForLoop::new_key_value(&node.key.clone().unwrap(), &node.value, container_val)
            },
            _ => bail!("Tried to iterate on a container (`{}`) that has a unsupported type", container_name)
        };

        let length = for_loop.len();

        match self.macro_context.last_mut() {
            Some(m) => m.1.push(for_loop),
            None => self.for_loops.push(for_loop)
        };

        let mut output = String::new();

        for _ in 0..length {
            output.push_str(&self.render_body(&node.body)?);
            // Safe unwrap
            match self.macro_context.last_mut() {
                Some(m) => m.1.last_mut().unwrap().increment(),
                None => self.for_loops.last_mut().unwrap().increment()
            };
        }
        // Clean up after ourselves
        match self.macro_context.last_mut() {
            Some(m) => m.1.pop(),
            None => self.for_loops.pop()
        };

        Ok(output)
    }

    /// Adds the macro for the given template into the renderer and returns
    /// whether it had some macros or not
    /// Used when rendering blocks
    fn import_template_macros(&mut self, tpl_name: &str) -> Result<bool> {
        let tpl = self.tera.get_template(tpl_name)?;
        if tpl.imported_macro_files.is_empty() {
            return Ok(false);
        }

        /// Macro templates can import other macro templates so the macro loading
        /// needs to happen recursively
        /// We need all of the macros loaded in one go to be in the same hashmap
        /// for easy popping as well, otherwise there could be stray macro definitions
        /// remaining
        fn load_macros<'a>(tera: &'a Tera, tpl: &Template) -> Result<HashMap<String, &'a HashMap<String, MacroDefinition>>> {
            let mut macros = HashMap::new();

            for &(ref filename, ref namespace) in &tpl.imported_macro_files {
                let macro_tpl = tera.get_template(filename)?;
                macros.insert(namespace.to_string(), &macro_tpl.macros);
                if !macro_tpl.imported_macro_files.is_empty() {
                    macros.extend(load_macros(tera, macro_tpl)?);
                }
            }

            Ok(macros)
        }

        self.macros.push(load_macros(self.tera, &tpl)?);

        Ok(true)
    }

    /// The way inheritance work is that the top parent will be rendered by the renderer so for blocks
    /// we want to look from the bottom (`level = 0`, the template the user is actually rendering)
    /// to the top (the base template).
    /// If we are rendering a block,
    fn render_block(&mut self, block: &Block, level: usize) -> Result<String> {
        let blocks_definitions = match level {
            // look for the template we're currently rendering
            0 => &self.template.blocks_definitions,
            // or look at its parents
            _ => &self.tera.get_template(&self.template.parents[level - 1]).unwrap().blocks_definitions,
        };

        // Can we find this one block in these definitions? If so render it
        if let Some(block_def) = blocks_definitions.get(&block.name) {
            let (ref tpl_name, Block {ref body, ..}) = block_def[0];
            self.blocks.push((block.name.to_string(), level));
            let has_macro = self.import_template_macros(tpl_name)?;
            let res = self.render_body(body);
            if has_macro {
                self.macros.pop();
            }
            return res;
        }

        // Do we have more parents to look through?
        if level + 1 <= self.template.parents.len() {
            return self.render_block(block, level + 1);
        }

        // Nope, just render the body we got
        self.render_body(&block.body)
    }

    /// Only called while rendering a block.
    /// This will look up the block we are currently rendering and its level and try to render
    /// the block at level + n, where would be the next template in the hierarchy the block is present
    fn do_super(&mut self) -> Result<String> {
        let (block_name, level) = self.blocks.pop().unwrap();
        let mut next_level = level + 1;

        while next_level <= self.template.parents.len() {
            let blocks_definitions = &self.tera.get_template(&self.template.parents[next_level - 1]).unwrap().blocks_definitions;

            if let Some(block_def) = blocks_definitions.get(&block_name) {
                let (ref tpl_name, Block { ref body, .. }) = block_def[0];
                self.blocks.push((block_name.to_string(), next_level));
                let has_macro = self.import_template_macros(tpl_name)?;
                let res = self.render_body(body);
                if has_macro {
                    self.macros.pop();
                }
                // Can't go any higher for that block anymore?
                if next_level >= self.template.parents.len() {
                    // then remove it from the stack, we're done with it
                    self.blocks.pop();
                }
                return res;
            } else {
                next_level += 1;
            }
        }

        bail!("Tried to use super() in the top level block")
    }

    fn render_node(&mut self, node: &Node) -> Result<String> {
        let output = match *node {
            Node::Text(ref s) | Node::Raw(_, ref s, _) => s.to_string(),
            Node::VariableBlock(ref expr) => self.eval_expression(expr)?.render(),
            Node::Set(_, ref set) => self.eval_set(set).and(Ok(String::new()))?,
            Node::FilterSection(_, FilterSection {ref filter, ref body}, _) => {
                let output = self.render_body(body)?;

                self.eval_filter(Value::String(output), filter)?.render()
            },
            // Macros have been imported at the beginning
            Node::ImportMacro(_, _, _) => String::new(),
            Node::If(ref if_node, _) => self.render_if(if_node)?,
            Node::Forloop(_, ref forloop, _) => self.render_for(forloop)?,
            Node::Block(_, ref block, _) => self.render_block(block, 0)?,
            Node::Super => self.do_super()?,
            Node::Include(_, ref tpl_name) => {
                let has_macro = self.import_template_macros(tpl_name)?;
                let res = self.render_body(&self.tera.get_template(tpl_name)?.ast);
                if has_macro {
                    self.macros.pop();
                }
                return res;
            },
            _ => unreachable!("render_node -> unexpected node: {:?}", node),
        };

        Ok(output)
    }

    fn render_body(&mut self, body: &[Node]) -> Result<String> {
        let mut output = String::new();

        for n in body {
            output.push_str(&self.render_node(n)?);
        }

        Ok(output)
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
            } else {
                error_location += " (error happened in a parent template)";
            }
        } else if let Some(parent) = self.template.parents.last() {
            // Error happened in the base template, outside of blocks
            error_location += &format!(" (error happened in '{}').", parent);
        }

        error_location
    }

    pub fn render(&mut self) -> Result<String> {
        // If we have a parent for the template, we start by rendering
        // the one at the top
        let (tpl_name, ast) = match self.template.parents.last() {
            // this unwrap is safe; Tera would have errored already if the template didn't exist
            Some(parent_tpl_name) => {
                let tpl = self.tera.get_template(parent_tpl_name).unwrap();
                (&tpl.name, &tpl.ast)
            },
            None => (&self.template.name, &self.template.ast),
        };

        self.import_template_macros(tpl_name)?;

        let mut output = String::new();
        for node in ast {
            output.push_str(
                &self.render_node(node).chain_err(|| self.get_error_location())?
            );
        }

        Ok(output)
    }
}
