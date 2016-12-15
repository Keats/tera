use std::collections::{LinkedList, HashMap};

use serde_json::to_string_pretty;
use serde_json::value::{Value, to_value};

use context::{ValueRender, ValueNumber, ValueTruthy, get_json_pointer};
use template::Template;
use errors::{Result, ResultExt};
use parser::Node;
use parser::Node::*;
use tera::Tera;
use utils::escape_html;



static MAGICAL_DUMP_VAR: &'static str = "__tera_context";

// we need to have some data in the renderer for when we are in a ForLoop
// For example, accessing the local variable would fail when
// looking it up in the context
#[derive(Debug)]
struct ForLoop {
    variable_name: String,
    current: usize,
    values: Vec<Value>
}

impl ForLoop {
    pub fn new(local: String, values: Vec<Value>) -> ForLoop {
        ForLoop {
            variable_name: local,
            current: 0,
            values: values
        }
    }

    pub fn increment(&mut self) {
        self.current += 1;
    }

    pub fn get(&self) -> Option<&Value> {
        self.values.get(self.current)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }
}

#[derive(Debug)]
pub struct Renderer<'a> {
    template: &'a Template,
    context: Value,
    tera: &'a Tera,
    for_loops: Vec<ForLoop>,
    // looks like Vec<filename: {macro_name: body node}>
    macros: Vec<HashMap<String, HashMap<String, Node>>>,
    // set when rendering macros, empty if not in a macro
    macro_context: Vec<Value>,
    // Keeps track of which namespace we're on in order to resolve the `self::` syntax
    macro_namespaces: Vec<String>,
    should_escape: bool,
    // Used when super() is used in a block, to know where we are in our stack of
    // definitions and for which block (block name, hierarchy level)
    blocks: Vec<(String, usize)>,
}

impl<'a> Renderer<'a> {
    pub fn new(tpl: &'a Template, tera: &'a Tera, context: Value) -> Renderer<'a> {
        let should_escape = tera.autoescape_extensions.iter().any(|ext| tpl.name.ends_with(ext));
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

    // Lookup a variable name from the context and takes into
    // account for loops variables
    fn lookup_variable(&self, key: &str) -> Result<Value> {
        // Differentiate between macros and general context
        let context = match self.macro_context.last() {
            Some(c) => c,
            None => &self.context
        };

        // Magical variable that just dumps the context
        if key == MAGICAL_DUMP_VAR {
            return Ok(to_value(
                to_string_pretty(context).expect("Couldn't serialize context for `__tera_context`")
            ));
        }

        // small helper fn to reduce duplication code in the 3 spots in `lookup_variable` where we
        // need to actually do the variable lookup
        fn find_variable(context: &Value, key: &str, tpl_name: &str) -> Result<Value> {
            match context.pointer(&get_json_pointer(key)).cloned() {
                Some(v) => Ok(v),
                None => bail!("Field `{}` not found in context while rendering '{}'", key, tpl_name)
            }
        }

        // Look in the plain context if we aren't in a for loop
        if self.for_loops.is_empty() {
            return find_variable(context, key, &self.template.name);
        }

        for for_loop in self.for_loops.iter().rev() {
            if key.starts_with(&for_loop.variable_name) {
                let value = match for_loop.get() {
                    Some(f) => f,
                    None => { return Ok(to_value(&"")); }
                };

                // might be a struct or some nested structure
                if key.contains('.') {
                    let new_key = key.split_terminator('.').skip(1).collect::<Vec<&str>>().join(".");
                    return find_variable(value, &new_key, &self.template.name);
                } else {
                    return Ok(value.clone());
                }
            } else {
                match key {
                    "loop.index" => { return Ok(to_value(&(for_loop.current + 1))); },
                    "loop.index0" => { return Ok(to_value(&for_loop.current)); },
                    "loop.first" => { return Ok(to_value(&(for_loop.current == 0))); },
                    "loop.last" => { return Ok(to_value(&(for_loop.current == for_loop.len() - 1))); },
                    _ => ()
                };
            }
        }

        // can get there when looking a variable in the global context while in a forloop
        find_variable(context, key, &self.template.name)
    }

    // Gets an identifier and return its json value
    // If there is no filter, it's itself, otherwise call the filters in order
    // an return their result
    fn eval_ident(&self, node: &Node) -> Result<Value> {
        match *node {
            Identifier { ref name, ref filters } => {
                let mut value = self.lookup_variable(name)?;
                let mut is_safe = false;

                if let Some(ref _filters) = *filters {
                    for filter in _filters {
                        match *filter {
                            Filter { ref name, ref params } => {
                                if name == "safe" {
                                    is_safe = true;
                                    continue;
                                }
                                let filter_fn = self.tera.get_filter(name)?;
                                let mut all_args = HashMap::new();
                                for (arg_name, exp) in params {
                                    all_args.insert(arg_name.to_string(), self.eval_expression(exp.clone())?);
                                }
                                value = filter_fn(value, all_args)?;
                            },
                            _ => unreachable!(),
                        };
                    }
                }

                // Escaping strings if wanted for that template
                if name != MAGICAL_DUMP_VAR && self.should_escape && !is_safe {
                    if let Value::String(s) = value {
                        value = to_value(escape_html(s.as_str()));
                    }
                }
                Ok(value)
            },
            _ => unreachable!()
        }
    }

    fn eval_math(&self, node: &Node) -> Result<f32> {
        match *node {
            Identifier { ref name, .. } => {
                self.eval_ident(node)?
                    .to_number()
                    .or(Err(
                        format!(
                            "Variable `{}` was used in a math operation but is not a number", name
                        ).into()
                    ))
            },
            Int(s) => Ok(s as f32),
            Float(s) => Ok(s),
            Math { ref lhs, ref rhs, ref operator } => {
                let l = self.eval_math(lhs)?;
                let r = self.eval_math(rhs)?;
                let mut result = match operator.as_str() {
                    "*" => l * r,
                    "/" => l / r,
                    "+" => l + r,
                    "-" => l - r,
                    _ => unreachable!()
                };
                // TODO: fix properly
                // TODO: add tests for float maths arithmetics
                if result.fract() < 0.01 {
                    result = result.round();
                }
                Ok(result)
            }
            Text(ref s) => bail!("Tried to do math with a String: `{}`", s),
            Bool(s) => bail!("Tried to do math with a boolean: `{}`", s),
            _ => unreachable!()
        }
    }

    fn eval_expression(&self, node: Node) -> Result<Value> {
        match node {
            Identifier { .. } => {
                Ok(self.eval_ident(&node)?)
            },
            l @ Logic { .. } => {
                let value = self.eval_condition(l)?;
                Ok(Value::Bool(value))
            },
            m @ Math { .. } => {
                let result = self.eval_math(&m)?;
                Ok(Value::F64(result as f64))
            },
            Int(val) => {
                Ok(Value::I64(val as i64))
            },
            Float(val) => {
                Ok(Value::F64(val as f64))
            },
            Bool(b) => {
                Ok(Value::Bool(b))
            },
            Text(t) => {
                Ok(Value::String(t))
            },
            _ => unreachable!()
        }
    }

    fn eval_condition(&self, node: Node) -> Result<bool> {
        match node {
            Identifier { .. } => {
                Ok(self.eval_ident(&node).map(|v| v.is_truthy()).unwrap_or(false))
            },
            Test { expression, name, params } => {
                let tester = self.tera.get_tester(&name)?;
                let mut value_params = vec![];
                for param in params {
                    value_params.push(self.eval_expression(param)?);
                }
                tester(self.eval_expression(*expression).ok(), value_params)
            },
            Logic { lhs, rhs, operator } => {
                match operator.as_str() {
                    "or" => {
                        let result = self.eval_condition(*lhs)? || self.eval_condition(*rhs)?;
                        Ok(result)
                    },
                    "and" => {
                        let result = self.eval_condition(*lhs)? && self.eval_condition(*rhs)?;
                        Ok(result)
                    },
                    ">=" | ">" | "<=" | "<" => {
                        let l = self.eval_math(&lhs)?;
                        let r = self.eval_math(&rhs)?;
                        let result = match operator.as_str() {
                            ">=" => l >= r,
                            ">" => l > r,
                            "<=" => l <= r,
                            "<" => l < r,
                            _ => unreachable!()
                        };
                        Ok(result)
                    },
                    "==" | "!=" => {
                        let mut lhs_val = self.eval_expression(*lhs)?;
                        let mut rhs_val = self.eval_expression(*rhs)?;

                        // Monomorphize number vals.
                        if lhs_val.is_number() || rhs_val.is_number() {
                            if !lhs_val.is_number() || !rhs_val.is_number() {
                                return Ok(false);
                            }

                            // Since Tera only support 32 bit integers, this
                            // actually preserves all of the precision. If Tera
                            // switches to 64-bit values, use std::f32::EPSILON
                            // to get an approximation as before.
                            lhs_val = Value::F64(lhs_val.as_f64().unwrap());
                            rhs_val = Value::F64(rhs_val.as_f64().unwrap());
                        }

                        let result = match operator.as_str() {
                            "==" => lhs_val == rhs_val,
                            "!=" => lhs_val != rhs_val,
                            _ => unreachable!()
                        };

                        Ok(result)
                    },
                    _ => unreachable!()
                }
            }
            _ => unreachable!()
        }
    }

    // eval all the values in a {{ }} block
    // Macro calls and super are NOT variable blocks in the AST, they have
    // their own nodes
    fn render_variable_block(&mut self, node: Node) -> Result<String>  {
        match node {
            Identifier { .. } => Ok(self.eval_ident(&node)?.render()),
            Math { .. } => Ok(self.eval_math(&node)?.to_string()),
            _ => unreachable!()
        }
    }

    // evaluates conditions and render bodies accordingly
    fn render_if(&mut self, condition_nodes: LinkedList<Node>, else_node: Option<Box<Node>>) -> Result<String> {
        let mut skip_else = false;
        let mut output = String::new();
        for node in condition_nodes {
            match node {
                Conditional {condition, body } => {
                    if self.eval_condition(*condition)? {
                        skip_else = true;
                        // Remove if/elif whitespace
                        output.push_str(self.render_node(*body.clone())?.trim_left());
                    }
                },
                _ => unreachable!()
            }
        }

        if skip_else {
            // Remove endif whitespace
            return Ok(output.trim_right().to_string());
        }

        if let Some(e) = else_node {
            // Remove else whitespace
            output.push_str(self.render_node(*e)?.trim_left());
        };

        // Remove endif whitespace
        Ok(output.trim_right().to_string())
    }

    fn render_for(&mut self, variable_name: String, array_name: String, body: Box<Node>) -> Result<String> {
        let list = self.lookup_variable(&array_name)?;

        if !list.is_array() {
            bail!("Tried to iterate on variable `{}`, but it isn't an array", array_name);
        }

        // Safe unwrap
        let deserialized = list.as_array().unwrap();
        let length = deserialized.len();
        self.for_loops.push(ForLoop::new(variable_name, deserialized.clone()));
        let mut i = 0;
        let mut output = String::new();
        if length > 0 {
            loop {
                output.push_str(self.render_node(*body.clone())?.trim_left());
                // Safe unwrap
                self.for_loops.last_mut().unwrap().increment();
                if i == length - 1 {
                    // Don't forget to pop the for_loop is we are done
                    // otherwise it would just replay the last loop
                    // see https://github.com/Keats/tera/issues/51
                    self.for_loops.pop();
                    break;
                }
                i += 1;
            }
            output = output.trim_right().to_string();
        } else {
            self.for_loops.pop();
        }

        Ok(output.trim_right().to_string())
    }

    fn render_macro(&mut self, call_node: Node) -> Result<String> {
        if let MacroCall {namespace, name: macro_name, params: call_params} = call_node {
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
                .and_then(|m| m.get(&macro_name)
                .cloned());

            if let Some(Macro {body, params, ..}) = macro_definition {
                // fail fast if the number of args don't match
                if params.len() != call_params.len() {
                    let params_seen = call_params.keys().cloned().collect::<Vec<String>>();
                    bail!("Macro `{}` got `{:?}` for args but was expecting `{:?}` (order does not matter)", macro_name, params, params_seen);
                }

                // We need to make a new context for the macro from the arguments given
                // Return an error if we get some unknown params
                let mut context = HashMap::new();
                for (param_name, exp) in call_params.clone() {
                    if !params.contains(&param_name) {
                        let params_seen = call_params.keys().cloned().collect::<Vec<String>>();
                        bail!("Macro `{}` got `{:?}` for args but was expecting `{:?}` (order does not matter)", macro_name, params, params_seen);
                    }
                    context.insert(param_name.to_string(), self.eval_expression(exp)?);
                }

                // Push this context to our stack of macro context so the renderer can pick variables
                // from it
                self.macro_context.push(to_value(&context));

                // We render the macro body as a normal node
                let mut output = String::new();
                for node in body.get_children() {
                    output.push_str(&self.render_node(node)?);
                }

                // If the current namespace wasn't `self`, we remove it since it's not needed anymore
                // In the `self` case, we are still in the parent macro and its namespace is still
                // needed so we keep it
                if namespace == active_namespace {
                    self.macro_namespaces.pop();
                }

                // We remove the macro context we just rendered from our stack of contexts
                self.macro_context.pop();

                return Ok(output.trim().to_string());
            } else {
                bail!("Macro `{}` was not found in the namespace `{}`", macro_name, active_namespace);
            }
        } else {
            unreachable!("Got a node other than a MacroCall when rendering a macro")
        }
    }

    fn import_macros(&mut self, tpl_name: String) -> Result<bool> {
        let tpl = self.tera.get_template(&tpl_name)?;
        if tpl.imported_macro_files.len() == 0 {
            return Ok(false);
        }
        let mut map = HashMap::new();

        for &(ref filename, ref namespace) in &tpl.imported_macro_files {
            let macro_tpl = self.tera.get_template(&filename)?;
            map.insert(namespace.to_string(), macro_tpl.macros.clone());
        }
        self.macros.push(map);
        Ok(true)
    }

    pub fn render_node(&mut self, node: Node) -> Result<String> {
        match node {
            Include(p) => {
                let ast = self.tera.get_template(&p)?.ast.get_children();
                let mut output = String::new();
                for node in ast {
                    output.push_str(&self.render_node(node)?);
                }

                Ok(output.trim_left().to_string())
            },
            ImportMacro {tpl_name, name} => {
                let tpl = self.tera.get_template(&tpl_name)?;
                let mut map = if self.macros.len() == 0 {
                    HashMap::new()
                } else {
                    self.macros.pop().unwrap()
                };
                map.insert(name.to_string(), tpl.macros.clone());
                self.macros.push(map);
                // In theory, the render_node should return Result<Option<String>>
                // but in practice there's no difference so keeping this hack
                Ok("".to_string())
            },
            MacroCall {..} => self.render_macro(node),
            Text(s) => Ok(s),
            Raw(s) => Ok(s.trim().to_string()),
            VariableBlock(exp) => self.render_variable_block(*exp),
            If {condition_nodes, else_node} => {
                self.render_if(condition_nodes, else_node)
            },
            List(body) => {
                let mut output = String::new();
                for n in body {
                    output.push_str(&self.render_node(n)?);
                }
                Ok(output)
            },
            For {variable, array, body} => {
                self.render_for(variable, array, body)
            },
            Block {name, body} => {
                // We pick the first block, ie the one in the template we are rendering
                // We will go up in "level" if we encounter a super()
                match self.template.blocks_definitions.get(&name) {
                    Some(b) => {
                        // the indexing here is safe since we are rendering a block, we know we have
                        // at least 1
                        match b[0].clone() {
                            (tpl_name, Block {body, ..}) => {
                                self.blocks.push((name.clone(), 0));
                                let has_macro = self.import_macros(tpl_name)?;
                                let res = self.render_node(*body.clone());
                                if has_macro {
                                    self.macros.pop();
                                }
                                res
                            },
                            x @ _ => unreachable!("render_node Block {:?}", x)
                        }
                    },
                    None => {
                        self.render_node(*body)
                    }
                }
            },
            Super => {
                if let Some((name, level)) = self.blocks.pop() {
                    let new_level = level + 1;

                    match self.template.blocks_definitions.get(&name) {
                        Some(b) => {
                            match b[new_level].clone() {
                                (tpl_name, Block { body, .. }) => {
                                    self.blocks.push((name.clone(), new_level));
                                    let has_macro = self.import_macros(tpl_name)?;
                                    let res = self.render_node(*body.clone());
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
                                x @ _ => unreachable!("render_node Block {:?}", x)
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
            Extends(_) => Ok("".to_string()),
            Macro {..} => Ok("".to_string()),
            x @ _ => unreachable!("render_node -> unexpected node: {:?}", x)
        }
    }

    // Helper fn that tries to find the current context: are we in a macro? in a parent template?
    // in order to give the best possible error when getting an error when rendering a tpl
    // TODO: find a way to write tests for that
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
                .and_then(|b| b.get(level + 1));

            if let Some(&(ref tpl_name, _)) = block_def {
                if tpl_name != &self.template.name {
                    error_location += &format!(" (error happened in '{}').", tpl_name);
                }
            }
        } else {
            // Error happened in the base template, outside of blocks
            if let Some(parent) = self.template.parents.last() {
                error_location += &format!(" (error happened in '{}').", parent);
            }
        }

        error_location
    }

    pub fn render(&mut self) -> Result<String> {
        let ast = if self.template.parents.len() > 0 {
            let parent = self.tera.get_template(
                &self.template.parents.last().expect("Couldn't get first ancestor template")
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
    use context::Context;
    use errors::Result;
    use tera::Tera;

    fn render_template(content: &str, context: Context) -> Result<String> {
        let mut tera = Tera::default();
        tera.add_template("hello", content).unwrap();

        tera.render("hello", context)
    }

    #[test]
    fn test_render_include() {
        let mut tera = Tera::default();
        tera.add_template("world", "world").unwrap();
        tera.add_template("hello", "<h1>Hello {% include \"world\" %}</h1>").unwrap();
        let result = tera.render("hello", Context::new());
        assert_eq!(result.unwrap(), "<h1>Hello world</h1>".to_owned());
    }

    #[test]
    fn test_render_simple_string() {
        let result = render_template("<h1>Hello world</h1>", Context::new());
        assert_eq!(result.unwrap(), "<h1>Hello world</h1>".to_owned());
    }

    #[test]
    fn test_render_math() {
        let result = render_template("This is {{ 2000 + 16 }}.", Context::new());
        assert_eq!(result.unwrap(), "This is 2016.".to_owned());
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
        tera.add_template("hello.html", "{{bad}}").unwrap();
        let result = tera.render("hello.html", context);

        assert_eq!(result.unwrap(), "&lt;script&gt;alert(&#x27;pwnd&#x27;);&lt;&#x2F;script&gt;".to_string());
    }

    #[test]
    fn test_no_autoescape_on_extensions_not_specified() {
        let mut context = Context::new();
        context.add("bad", &"<script>alert('pwnd');</script>");
        let mut tera = Tera::default();
        tera.add_template("hello.sql", "{{bad}}").unwrap();
        let result = tera.render("hello.sql", context);

        assert_eq!(result.unwrap(), "<script>alert('pwnd');</script>".to_string());
    }

    #[test]
    fn test_no_autoescape_with_safe_filter() {
        let mut context = Context::new();
        context.add("bad", &"<script>alert('pwnd');</script>");
        let mut tera = Tera::default();
        tera.add_template("hello.html", "{{ bad | safe }}").unwrap();
        let result = tera.render("hello.html", context);

        assert_eq!(result.unwrap(), "<script>alert('pwnd');</script>".to_string());
    }

    #[test]
    fn test_render_super_multiple_inheritance() {
        let mut tera = Tera::default();
        tera.add_templates(vec![
            ("grandparent", "{% block hey %}hello{% endblock hey %} {% block ending %}sincerely{% endblock ending %}"),
            ("parent", "{% extends \"grandparent\" %}{% block hey %}hi and grandma says {{ super() }}{% endblock hey %}"),
            ("child", "{% extends \"parent\" %}{% block hey %}dad says {{ super() }}{% endblock hey %}{% block ending %}{{ super() }} with love{% endblock ending %}"),
        ]).unwrap();
        let result = tera.render("child", Context::new());

        assert_eq!(result.unwrap(), "dad says hi and grandma says hello sincerely with love".to_string());
    }

    #[test]
    fn test_render_super_multiple_inheritance_nested_block() {
        let mut tera = Tera::default();
        tera.add_templates(vec![
            ("grandparent", "{% block hey %}hello{% endblock hey %}"),
            ("parent", "{% extends \"grandparent\" %}{% block hey %}hi and grandma says {{ super() }} {% block ending %}sincerely{% endblock ending %}{% endblock hey %}"),
            ("child", "{% extends \"parent\" %}{% block hey %}dad says {{ super() }}{% endblock hey %}{% block ending %}{{ super() }} with love{% endblock ending %}"),
        ]).unwrap();
        let result = tera.render("child", Context::new());

        assert_eq!(result.unwrap(), "dad says hi and grandma says hello sincerely with love".to_string());
    }

    #[test]
    fn test_render_macros() {
        let mut tera = Tera::default();
        tera.add_templates(vec![
            ("macros", "{% macro hello()%}Hello{% endmacro hello %}"),
            ("tpl", "{% import \"macros\" as macros %}{% block hey %}{{macros::hello()}}{% endblock hey %}"),
        ]).unwrap();

        let result = tera.render("tpl", Context::new());

        assert_eq!(result.unwrap(), "Hello".to_string());
    }

    #[test]
    fn test_render_macros_in_child_templates_same_namespace() {
        let mut tera = Tera::default();
        tera.add_templates(vec![
            ("grandparent", "{% block hey %}hello{% endblock hey %}"),
            ("macros", "{% macro hello()%}Hello{% endmacro hello %}"),
            ("macros2", "{% macro hi()%}Hi{% endmacro hi %}"),
            ("parent", "{% extends \"grandparent\" %}{% import \"macros\" as macros %}{% block hey %}{{macros::hello()}}{% endblock hey %}"),
            ("child", "{% extends \"parent\" %}{% import \"macros2\" as macros %}{% block hey %}{{super()}}/{{macros::hi()}}{% endblock hey %}"),
        ]).unwrap();

        let result = tera.render("child", Context::new());

        assert_eq!(result.unwrap(), "Hello/Hi".to_string());
    }

    #[test]
    fn test_render_macros_in_child_templates_different_namespace() {
        let mut tera = Tera::default();
        tera.add_templates(vec![
            ("grandparent", "{% block hey %}hello{% endblock hey %}"),
            ("macros", "{% macro hello()%}Hello{% endmacro hello %}"),
            ("macros2", "{% macro hi()%}Hi{% endmacro hi %}"),
            ("parent", "{% extends \"grandparent\" %}{% import \"macros\" as macros %}{% block hey %}{{macros::hello()}}{% endblock hey %}"),
            ("child", "{% extends \"parent\" %}{% import \"macros2\" as macros2 %}{% block hey %}{{super()}}/{{macros2::hi()}}{% endblock hey %}"),
        ]).unwrap();

        let result = tera.render("child", Context::new());

        assert_eq!(result.unwrap(), "Hello/Hi".to_string());
    }

    #[test]
    fn test_render_macros_in_parent_template_with_inheritance() {
        let mut tera = Tera::default();
        tera.add_templates(vec![
            ("macros", "{% macro hello()%}Hello{% endmacro hello %}"),
            ("grandparent", "{% import \"macros\" as macros %}{% block hey %}{{macros::hello()}}{% endblock hey %}"),
            ("child", "{% extends \"grandparent\" %}{% import \"macros\" as macros %}{% block hey %}{{super()}}/{{macros::hello()}}{% endblock hey %}"),
        ]).unwrap();

        let result = tera.render("child", Context::new());

        assert_eq!(result.unwrap(), "Hello/Hello".to_string());
    }

    #[test]
    fn test_error_location_basic() {
        let mut tera = Tera::default();
        tera.add_templates(vec![
            ("tpl", "{{ 1 + true }}"),
        ]).unwrap();

        let result = tera.render("tpl", Context::new());

        assert_eq!(
            result.unwrap_err().iter().nth(0).unwrap().description(),
            "Failed to render \'tpl\'"
        );
    }

    #[test]
    fn test_error_location_inside_macro() {
        let mut tera = Tera::default();
        tera.add_templates(vec![
            ("macros", "{% macro hello()%}{{ 1 + true }}{% endmacro hello %}"),
            ("tpl", "{% import \"macros\" as macros %}{{ macro::hello() }}"),
        ]).unwrap();

        let result = tera.render("tpl", Context::new());

        assert_eq!(
            result.unwrap_err().iter().nth(0).unwrap().description(),
            "Failed to render \'tpl\': error while rendering a macro from the `macro` namespace"
        );
    }

    #[test]
    fn test_error_location_base_template() {
        let mut tera = Tera::default();
        tera.add_templates(vec![
            ("parent", "Hello {{ greeting + 1}} {% block bob %}{% endblock bob %}"),
            ("child", "{% extends \"parent\" %}{% block bob %}Hey{% endblock bob %}"),
        ]).unwrap();

        let result = tera.render("child", Context::new());

        assert_eq!(
            result.unwrap_err().iter().nth(0).unwrap().description(),
            "Failed to render \'child\' (error happened in 'parent')."
        );
    }

    #[test]
    fn test_error_location_in_parent_block() {
        let mut tera = Tera::default();
        tera.add_templates(vec![
            ("parent", "Hello {{ greeting }} {% block bob %}{{ 1 + true }}{% endblock bob %}"),
            ("child", "{% extends \"parent\" %}{% block bob %}{{ super() }}Hey{% endblock bob %}"),
        ]).unwrap();

        let result = tera.render("child", Context::new());

        assert_eq!(
            result.unwrap_err().iter().nth(0).unwrap().description(),
            "Failed to render \'child\' (error happened in 'parent')."
        );
    }

    #[test]
    fn test_error_location_in_parent_in_macro() {
        let mut tera = Tera::default();
        tera.add_templates(vec![
            ("macros", "{% macro hello()%}{{ 1 + true }}{% endmacro hello %}"),
            ("parent", "{% import \"macros\" as macros %}{{ macro::hello() }}{% block bob %}{% endblock bob %}"),
            ("child", "{% extends \"parent\" %}{% block bob %}{{ super() }}Hey{% endblock bob %}"),
        ]).unwrap();

        let result = tera.render("child", Context::new());

        assert_eq!(
            result.unwrap_err().iter().nth(0).unwrap().description(),
            "Failed to render \'child\': error while rendering a macro from the `macro` namespace (error happened in \'parent\')."
        );
    }
}
