//! Responsible for walking ast and render implementation

// --- module use statements ---

use context::{ValueRender, ValueTruthy};
use errors::{Result, ResultExt};
use parser::ast::{
    Block, Expr, ExprVal, FilterSection, Forloop, FunctionCall, If, LogicExpr, LogicOperator,
    MacroCall, MacroDefinition, MathExpr, MathOperator, Node, Set, Test,
};
use renderer::call_stack::{CallStack, FrameContext, FrameType};
use renderer::context::Context;
use renderer::for_loop::{ForLoop, ForLoopState};
use renderer::ref_or_owned::RefOrOwned;
use renderer::tera_macro::MacroCollection;
use serde_json::map::Map as JsonMap;
use serde_json::{to_value, Number, Value};
use std::collections::HashMap;
use template::Template;
use tera::Tera;
use utils::escape_html;

// --- module type aliases ---

type RenderResult = Result<String>;
type EvalResult<'a> = Result<RefOrOwned<'a, Value>>;
type LookupResult<'a> = Result<RefOrOwned<'a, Value>>;

// --- module statics ---

/// Special string indicating request to dump context
static MAGICAL_DUMP_VAR: &'static str = "__tera_context";

// --- module struct definitions ---

/// Processes the ast and renders the output
pub struct AstProcessor<'a> {
    /// The tera object with template details
    tera: &'a Tera,
    /// The call stack for processing
    call_stack: CallStack<'a>,
    /// The macro details
    macro_collection: MacroCollection<'a>,
    /// If set rendering should be escaped
    should_escape: bool,
    /// Tracks current active template
    template_stack: Vec<&'a Template>,
    /// Used when super() is used in a block, to know where we are in our stack of
    /// definitions and for which block
    /// Vec<(block name, level)>
    ///
    blocks: Vec<(String, usize)>,
}
/// Implementation for type `AstProcessor`.
impl<'a> AstProcessor<'a> {
    /// Create a new `AstProcessor`
    ///
    ///  * `tera` - The tera object with template details
    ///  * `template` - The template being processed
    ///  * `call_stack` - The call stack
    ///  * `macro_collection` - The macro details
    ///  * `should_escape` - If template should be escaped
    ///  * _return_ - Created `AstProcessor`
    ///
    pub fn new(
        tera: &'a Tera,
        template: &'a Template,
        call_stack: CallStack<'a>,
        macro_collection: MacroCollection<'a>,
        should_escape: bool,
    ) -> AstProcessor<'a> {
        AstProcessor {
            tera,
            call_stack,
            macro_collection,
            should_escape,
            template_stack: vec![template],
            blocks: Vec::new(),
        }
    }

    /// Takes the MacroCollection.
    ///
    /// Original `MacroCollection` has processed only top level file.
    /// More macros may have been added based on processing.
    /// This allows renderer to reuse any work.
    ///
    ///  * _return_ - The macro collection
    ///
    #[inline]
    pub fn take_macro_collection(self: &mut Self) -> MacroCollection<'a> {
        self.macro_collection.take_macro_collection()
    }

    /// Looks up identifier and returns its value
    ///
    ///  * `key` - Key to look up
    ///  * _return_ - Value if found
    ///
    pub fn lookup_ident(&self, key: &'a str) -> LookupResult<'a> {
        // custom <fn ast_processor_lookup_ident>

        info!("LOOKUP {}", key);

        let found = self.call_stack.find_value(key);
        info!(
            "Looking up `{}` in {:#?} -> {:?}",
            key, self.call_stack, found
        );

        match found {
            Some(v) => {
                info!("Found value for {}", key);
                Ok(v)
            }
            None => {
                warn!(
                    "Variable `{}` not found in context while rendering `{}`\n{}",
                    key,
                    self.template_stack.last().expect("Last template").name,
                    self.call_stack.debug_context()
                );

                bail!(
                    "Variable `{}` not found in context while rendering '{}'",
                    key,
                    self.template_stack.last().expect("Last template").name
                )
            }
        }

        // end <fn ast_processor_lookup_ident>
    }

    /// Walk the ast and render
    ///
    ///  * `ast` - `ast` to render
    ///  * _return_ - Resulting `ast` rendering
    ///
    pub fn render_ast(self: &mut Self, ast: &'a Vec<Node>) -> RenderResult {
        ast.announce_render();
        // custom <fn ast_processor_render_ast>

        let mut output = String::new();

        for node in ast {
            output.push_str(&self.render_node(node)
                .chain_err(|| "TODO: error location".to_string())?);
        }

        Ok(output)

        // end <fn ast_processor_render_ast>
    }

    /// Render for body
    ///
    ///  * `body` - `body` to render
    ///  * _return_ - Resulting `body` rendering
    ///
    pub fn render_body(self: &mut Self, body: &'a [Node]) -> RenderResult {
        body.announce_render();
        // custom <fn ast_processor_render_body>

        let mut result = String::with_capacity(body.len() * 16);

        for node in body {
            result.push_str(&self.render_node(node)?);
            if self.call_stack.should_break_body() {
                break;
            }
        }

        Ok(result)

        // end <fn ast_processor_render_body>
    }

    /// Render for for_loop
    ///
    ///  * `for_loop` - `for_loop` to render
    ///  * _return_ - Resulting `for_loop` rendering
    ///
    pub fn render_for_loop(self: &mut Self, for_loop: &'a Forloop) -> RenderResult {
        for_loop.announce_render();
        // custom <fn ast_processor_render_for_loop>

        let container_name = match for_loop.container.val {
            ExprVal::Ident(ref ident) => ident,
            ExprVal::FunctionCall(FunctionCall { ref name, .. }) => name,
            ExprVal::Array(_) => "an array literal",
            _ => bail!(
                "Forloop containers have to be an ident or a function call (tried to iterate on '{:?}')",
                for_loop.container.val,
            ),
        };

        let container_val = self.eval_expression_safe(&for_loop.container)?;

        let for_loop_name = &for_loop.value[..];
        let for_loop_body = &for_loop.body;

        let for_loop = match container_val.get() {
            Value::Array(array) => {
                if for_loop.key.is_some() {
                    bail!(
                        "Tried to iterate using key value on variable `{}`, but it isn't an object/map",
                        container_name,
                    );
                }
                ForLoop::from_array(&for_loop.value[..], container_val.clone())
            }
            Value::Object(_) => {
                if for_loop.key.is_none() {
                    bail!(
                        "Tried to iterate using key value on variable `{}`, but it is missing a key",
                        container_name,
                    );
                }
                ForLoop::from_object(
                    &for_loop.key.as_ref().unwrap(),
                    &for_loop.value[..],
                    if let Some(value) = container_val.get_ref() {
                        value
                    } else {
                        bail!(
                            "Key value iteration only available on borrowed objects: {}",
                            container_name,
                        )
                    },
                )
            }
            _ => bail!(
                "Tried to iterate on a container (`{}`) that has a unsupported type",
                container_name,
            ),
        };

        let len = for_loop.len();
        self.call_stack.push_for_loop_frame(for_loop_name, for_loop);

        let mut output = String::new();
        for _ in 0..len {
            output.push_str(&self.render_body(&for_loop_body)?);

            if self.call_stack.should_break_body() {
                break;
            }

            self.call_stack.increment_for_loop();
        }

        self.call_stack.pop_frame();

        Ok(output)

        // end <fn ast_processor_render_for_loop>
    }

    /// Render for if_node
    ///
    ///  * `if_node` - `if_node` to render
    ///  * _return_ - Resulting `if_node` rendering
    ///
    pub fn render_if_node(self: &mut Self, if_node: &'a If) -> RenderResult {
        if_node.announce_render();
        // custom <fn ast_processor_render_if_node>

        for &(_, ref expr, ref body) in &if_node.conditions {
            if self.eval_as_bool(expr)? {
                return self.render_body(body);
            }
        }

        if let Some((_, ref body)) = if_node.otherwise {
            return self.render_body(body);
        }

        Ok(String::new())

        // end <fn ast_processor_render_if_node>
    }

    /// Render for node
    ///
    ///  * `node` - `node` to render
    ///  * _return_ - Resulting `node` rendering
    ///
    pub fn render_node(self: &mut Self, node: &'a Node) -> RenderResult {
        node.announce_render();
        // custom <fn ast_processor_render_node>

        let output = match *node {
            Node::Text(ref s) | Node::Raw(_, ref s, _) => s.to_string(),
            Node::VariableBlock(ref expr) => self.eval_expression(expr)?.render(),
            Node::Set(_, ref set) => self.eval_set(set).and(Ok(String::new()))?,
            Node::FilterSection(
                _,
                FilterSection {
                    ref filter,
                    ref body,
                },
                _,
            ) => {
                let output = self.render_body(body)?;
                self.eval_filter(&RefOrOwned::from_owned(Value::String(output)), filter)?
                    .render()
            }
            // Macros have been imported at the beginning
            Node::ImportMacro(_, ref file, ref s) => String::new(),
            Node::If(ref if_node, _) => self.render_if_node(if_node)?,
            Node::Forloop(_, ref forloop, _) => self.render_for_loop(forloop)?,
            Node::Break(_) => {
                self.call_stack.break_for_loop();
                String::new()
            }
            Node::Continue(_) => {
                self.call_stack.continue_for_loop();
                String::new()
            }
            Node::Block(_, ref block, _) => self.render_block(block, 0)?,
            Node::Super => {
                "Super".into()
                // TODO: self.do_super()?,
            }
            Node::Include(_, ref tpl_name) => {
                "Include".into()
                // TODO
                // let has_macro = self.import_template_macros(tpl_name)?;
                // let res = self.render_body(&self.tera.get_template(tpl_name)?.ast);
                // if has_macro {
                //     self.macros.pop();
                // }
                // return res;
            }
            _ => unreachable!("render_node -> unexpected node: {:?}", node),
        };

        Ok(output)

        // end <fn ast_processor_render_node>
    }

    /// Render for block.
    ///
    /// The way inheritance work is that the top parent will be rendered by the renderer so for blocks
    /// we want to look from the bottom (`level = 0`, the template the user is actually rendering)
    /// to the top (the base template).
    /// If we are rendering a block,
    ///
    ///  * `block` - Render for block
    ///  * `level` - Level of inheritance
    ///  * _return_ - Resulting rendering
    ///
    pub fn render_block(self: &mut Self, block: &'a Block, level: usize) -> RenderResult {
        // custom <fn ast_processor_render_block>

        block.announce_render();

        let blocks_definitions = match level {
            0 => &self.call_stack.active_template().blocks_definitions,
            _ => {
                &self.tera
                    .get_template(&self.call_stack.active_template().parents[level - 1])
                    .unwrap()
                    .blocks_definitions
            }
        };

        // Can we find this one block in these definitions? If so render it
        if let Some(block_def) = blocks_definitions.get(&block.name) {
            info!("Found block {} -> {:#?}", block.name, block_def);
            let (ref tpl_name, Block { ref body, .. }) = block_def[0];
            self.blocks.push((block.name.to_string(), level));
            return self.render_body(body);
        /* TODO
            let has_macro = self.macro_collection. import_template_macros(tpl_name)?;
            let res = self.render_body(body);
            if has_macro {
                self.macros.pop();
            }
            return res;
            */
        } else {
            info!("Missing block {} in level {}", block.name, level);
        }

        // Do we have more parents to look through?
        if level + 1 <= self.call_stack.active_template().parents.len() {
            return self.render_block(block, level + 1);
        }

        // Nope, just render the body we got
        self.render_body(&block.body)

        // end <fn ast_processor_render_block>
    }

    /// Render for expression
    ///
    ///  * `expr` - Render for expression
    ///  * _return_ - Resulting evaluation
    ///
    pub fn eval_expression(self: &mut Self, expr: &'a Expr) -> EvalResult<'a> {
        expr.announce_eval();
        // custom <fn ast_processor_eval_expression>

        let mut needs_escape = false;

        let mut res = match expr.val {
            ExprVal::Array(ref arr) => {
                let mut vals = vec![];
                for v in arr {
                    vals.push(self.eval_expression(v)?);
                }
                RefOrOwned::from_owned(to_value(vals)?)
            }
            ExprVal::String(ref val) => {
                needs_escape = true;
                RefOrOwned::from_owned(Value::String(val.to_string()))
            }
            ExprVal::Int(val) => RefOrOwned::from_owned(Value::Number(val.into())),
            ExprVal::Float(val) => {
                RefOrOwned::from_owned(Value::Number(Number::from_f64(val).unwrap()))
            }
            ExprVal::Bool(val) => RefOrOwned::from_owned(Value::Bool(val)),
            ExprVal::Ident(ref ident) => {
                needs_escape = ident != MAGICAL_DUMP_VAR;
                // Negated idents are special cased as `not undefined_ident` should not
                // error but instead be falsy values
                match self.lookup_ident(ident) {
                    Ok(val) => val.clone(),
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
                            return Ok(RefOrOwned::from_owned(Value::Bool(true)));
                        }
                    }
                }
            }
            ExprVal::FunctionCall(ref fn_call) => {
                needs_escape = true;
                self.eval_global_fn_call(fn_call)?
            }
            ExprVal::MacroCall(ref macro_call) => {
                RefOrOwned::from_owned(Value::String(self.eval_macro_call(macro_call)?))
            }
            ExprVal::Test(ref test) => RefOrOwned::from_owned(Value::Bool(self.eval_test(test)?)),
            ExprVal::Logic(_) => RefOrOwned::from_owned(Value::Bool(self.eval_as_bool(expr)?)),
            ExprVal::Math(_) => {
                let result = self.eval_as_number(&expr.val)?;

                match Number::from_f64(result) {
                    Some(x) => RefOrOwned::from_owned(Value::Number(x)),
                    None => RefOrOwned::from_owned(Value::String("NaN".to_string())),
                }
            }
            _ => unreachable!("{:?}", expr),
        };

        info!(
            "Evaluated is: {:?}, should_escape({}) is_string({})",
            res,
            self.should_escape,
            res.is_string()
        );

        // Checks if it's a string and we need to escape it (if the first filter is `safe` we don't)
        if self.should_escape && needs_escape && res.is_string()
            && expr.filters.first().map_or(true, |f| f.name != "safe")
        {
            res = RefOrOwned::from_owned(to_value(escape_html(res.as_str().unwrap()))?);
        }

        for filter in &expr.filters {
            if filter.name == "safe" || filter.name == "default" {
                continue;
            }
            res = self.eval_filter(&res, filter)?;
        }

        // Lastly, we need to check if the expression is negated, thus turning it into a bool
        if expr.negated {
            return Ok(RefOrOwned::from_owned(Value::Bool(!res.is_truthy())));
        }

        Ok(res)

        // end <fn ast_processor_eval_expression>
    }

    /// Render for expression_safe
    ///
    ///  * `expr` - Render for expression_safe
    ///  * _return_ - Resulting evaluation
    ///
    pub fn eval_expression_safe(self: &mut Self, expr: &'a Expr) -> EvalResult<'a> {
        expr.announce_eval();
        // custom <fn ast_processor_eval_expression_safe>

        let should_escape = self.should_escape;
        self.should_escape = false;
        let res = self.eval_expression(expr);
        self.should_escape = should_escape;
        res

        // end <fn ast_processor_eval_expression_safe>
    }

    /// Render for global_fn_call
    ///
    ///  * `function_call` - Render for global_fn_call
    ///  * _return_ - Resulting evaluation
    ///
    pub fn eval_global_fn_call(self: &mut Self, function_call: &'a FunctionCall) -> EvalResult<'a> {
        function_call.announce_eval();
        // custom <fn ast_processor_eval_global_fn_call>

        let global_fn = self.tera.get_global_function(&function_call.name)?;

        let mut args = HashMap::new();
        for (arg_name, expr) in &function_call.args {
            args.insert(
                arg_name.to_string(),
                self.eval_expression_safe(expr)?.get().clone(),
            );
        }

        Ok(RefOrOwned::from_owned(global_fn(args)?.take()))

        // end <fn ast_processor_eval_global_fn_call>
    }

    /// Render for macro_call
    ///
    ///  * `macro_call` - Render for macro_call
    ///  * _return_ - Resulting evaluation
    ///
    pub fn eval_macro_call(self: &mut Self, macro_call: &'a MacroCall) -> Result<String> {
        macro_call.announce_eval();
        // custom <fn ast_processor_eval_macro_call>

        let mut active_template = self.call_stack.active_template();

        if macro_call.namespace != "self" {
            let mut found = false;
            info!(
                "Looking for namespace {} in [{:?}]",
                macro_call.namespace, &active_template.imported_macro_files
            );
            let imported_macro_files = &active_template.imported_macro_files;
            for (filename, namespace) in imported_macro_files {
                if macro_call.namespace == *namespace {
                    active_template = self.tera.get_template(&filename).expect("Expect template");
                    found = true;
                    break;
                }
            }

            if !found {
                bail!(
                    "Namespace `{}` not found in template `{}`",
                    macro_call.namespace,
                    active_template.name
                );
            }
        }

        info!(
            "Pushing new template {} to {:?}",
            active_template.name,
            self.template_stack
                .iter()
                .map(|t| &t.name[..])
                .collect::<Vec<&str>>()
        );

        let macro_definition = active_template
            .macros
            .get(&macro_call.name)
            .ok_or_else(|| {
                format!(
                    "Macro `{}` was not found in the namespace `{}` of template `{}`",
                    macro_call.name, macro_call.namespace, active_template.name,
                )
            })?;

        let mut frame_context = FrameContext::with_capacity(macro_definition.args.len());

        // First the default arguments
        for (arg_name, default_value) in &macro_definition.args {
            let value = match macro_call.args.get(arg_name) {
                Some(val) => self.eval_expression_safe(val)?,
                None => match *default_value {
                    Some(ref val) => self.eval_expression_safe(val)?,
                    None => bail!(
                        "Macro `{}` is missing the argument `{}`",
                        macro_call.name,
                        arg_name,
                    ),
                },
            };
            info!(
                "Adding macro arg {} for macro {}::{}",
                arg_name, macro_call.namespace, macro_call.name
            );

            frame_context.insert(&arg_name[..], value);
        }

        info!("In call {} => args {:#?}", macro_call.name, frame_context);

        info!(
            "Pushing macro {} with context {:?}",
            macro_call.name,
            frame_context.keys()
        );
        self.call_stack
            .push_macro_frame(&macro_call.name[..], frame_context, active_template);

        let output = self.render_body(&macro_definition.body)?;

        self.call_stack.pop_frame();

        info!("Popped macro frame {}", &macro_call.name[..],);

        Ok(output)

        // end <fn ast_processor_eval_macro_call>
    }

    /// Render for set
    ///
    ///  * `set` - Render for set
    ///  * _return_ - Resulting evaluation
    ///
    pub fn eval_set(self: &mut Self, set: &'a Set) -> Result<()> {
        set.announce_eval();
        // custom <fn ast_processor_eval_set>

        let assigned_value = self.eval_expression_safe(&set.value)?;
        self.call_stack.add_assignment(&set.key[..], assigned_value);

        Ok(())

        // end <fn ast_processor_eval_set>
    }

    /// Render for test
    ///
    ///  * `test` - Render for test
    ///  * _return_ - Resulting evaluation
    ///
    pub fn eval_test(self: &mut Self, test: &'a Test) -> Result<bool> {
        test.announce_eval();
        // custom <fn ast_processor_eval_test>

        let tester_fn = self.tera.get_tester(&test.name)?;

        let mut tester_args = vec![];
        for arg in &test.args {
            println!("Pushing test arg {:?}", arg);
            tester_args.push(self.eval_expression_safe(arg)?.get().clone());
        }

        let found = self.lookup_ident(&test.ident)
            .map(|found| found.get().clone())
            .ok();

        Ok(tester_fn(found, tester_args)?)

        // end <fn ast_processor_eval_test>
    }

    /// Evaluate filter on value
    ///
    ///  * `value` - Value to pass to filter
    ///  * `function_call` - Filter function
    ///  * _return_ - Resulting evaluation
    ///
    pub fn eval_filter(
        &mut self,
        value: &RefOrOwned<'a, Value>,
        function_call: &'a FunctionCall,
    ) -> EvalResult<'a> {
        // custom <fn ast_processor_eval_filter>
        function_call.announce_eval();

        let filter_fn = self.tera.get_filter(&function_call.name)?;

        let mut args = HashMap::new();
        for (arg_name, expr) in &function_call.args {
            args.insert(
                arg_name.to_string(),
                self.eval_expression_safe(expr)?.get().clone(),
            );
        }

        Ok(RefOrOwned::from_owned(filter_fn(
            value.get().clone(),
            args,
        )?))

        // end <fn ast_processor_eval_filter>
    }

    /// Evaluate expression as bool
    ///
    ///  * `bool_expr` - Boolean expression
    ///  * _return_ - Resulting evaluation
    ///
    pub fn eval_as_bool(&mut self, bool_expr: &'a Expr) -> Result<bool> {
        // custom <fn ast_processor_eval_as_bool>

        let res = match bool_expr.val {
            ExprVal::Logic(LogicExpr {
                ref lhs,
                ref rhs,
                ref operator,
            }) => {
                match *operator {
                    LogicOperator::Or => self.eval_as_bool(lhs)? || self.eval_as_bool(rhs)?,
                    LogicOperator::And => self.eval_as_bool(lhs)? && self.eval_as_bool(rhs)?,
                    LogicOperator::Gt
                    | LogicOperator::Gte
                    | LogicOperator::Lt
                    | LogicOperator::Lte => {
                        let l = self.eval_expr_as_number(lhs)?;
                        let r = self.eval_expr_as_number(rhs)?;

                        match *operator {
                            LogicOperator::Gte => l >= r,
                            LogicOperator::Gt => l > r,
                            LogicOperator::Lte => l <= r,
                            LogicOperator::Lt => l < r,
                            _ => unreachable!(),
                        }
                    }
                    LogicOperator::Eq | LogicOperator::NotEq => {
                        let mut lhs_val = self.eval_expression(lhs)?;
                        let mut rhs_val = self.eval_expression(rhs)?;

                        // Monomorphize number vals.
                        if lhs_val.is_number() || rhs_val.is_number() {
                            // We're not implementing JS so can't compare things of different types
                            if !lhs_val.is_number() || !rhs_val.is_number() {
                                return Ok(false);
                            }

                            lhs_val = RefOrOwned::from_owned(Value::Number(
                                Number::from_f64(lhs_val.as_f64().unwrap()).unwrap(),
                            ));
                            rhs_val = RefOrOwned::from_owned(Value::Number(
                                Number::from_f64(rhs_val.as_f64().unwrap()).unwrap(),
                            ));
                        }

                        match *operator {
                            LogicOperator::Eq => *lhs_val == *rhs_val,
                            LogicOperator::NotEq => *lhs_val != *rhs_val,
                            _ => unreachable!(),
                        }
                    }
                }
            }
            ExprVal::Ident(ref ident) => self.lookup_ident(ident)
                .map(|v| v.is_truthy())
                .unwrap_or(false),
            ExprVal::Math(_) | ExprVal::Int(_) | ExprVal::Float(_) => self.eval_as_number(
                &bool_expr.val,
            ).map(|v| v != 0.0 && !v.is_nan())?,
            ExprVal::Test(ref test) => self.eval_test(test).unwrap_or(false),
            ExprVal::Bool(val) => val,
            ExprVal::String(ref string) => !string.is_empty(),
            _ => unreachable!("unimplemented logic operation for {:?}", bool_expr),
        };

        if bool_expr.negated {
            return Ok(!res);
        }

        Ok(res)

        // end <fn ast_processor_eval_as_bool>
    }

    /// Evaluate expression value as number, monomorphing to f64
    ///
    ///  * `bool_expr` - Expression to evaluate as number normalized to f64
    ///  * _return_ - Resulting evaluation
    ///
    pub fn eval_expr_as_number(&mut self, bool_expr: &'a Expr) -> Result<f64> {
        // custom <fn ast_processor_eval_expr_as_number>

        if !bool_expr.filters.is_empty() {
            match self.eval_expression(bool_expr)?.get() {
                Value::Number(s) => Ok(s.as_f64().unwrap()),
                _ => bail!("Tried to do math with an expression not resulting in a number"),
            }
        } else {
            self.eval_as_number(&bool_expr.val)
        }

        // end <fn ast_processor_eval_expr_as_number>
    }

    /// Evaluate expression as number, monomorphing to f64
    ///
    ///  * `expr_val` - Expression to evaluate as number normalized to f64
    ///  * _return_ - Resulting evaluation
    ///
    pub fn eval_as_number(&mut self, expr_val: &'a ExprVal) -> Result<f64> {
        // custom <fn ast_processor_eval_as_number>

        let res = match *expr_val {
            ExprVal::Ident(ref ident) => match self.lookup_ident(ident)?.as_f64() {
                Some(v) => v,
                None => bail!(
                    "Variable `{}` was used in a math operation but is not a number",
                    ident,
                ),
            },
            ExprVal::Int(val) => val as f64,
            ExprVal::Float(val) => val,
            ExprVal::Math(MathExpr {
                ref lhs,
                ref rhs,
                ref operator,
            }) => {
                let l = self.eval_expr_as_number(lhs)?;
                let r = self.eval_expr_as_number(rhs)?;
                match *operator {
                    MathOperator::Mul => l * r,
                    MathOperator::Div => l / r,
                    MathOperator::Add => l + r,
                    MathOperator::Sub => l - r,
                    MathOperator::Modulo => l % r,
                }
            }
            ExprVal::String(ref val) => bail!("Tried to do math with a string: `{}`", val),
            ExprVal::Bool(val) => bail!("Tried to do math with a boolean: `{}`", val),
            _ => unreachable!("unimplemented"),
        };

        Ok(res)

        // end <fn ast_processor_eval_as_number>
    }

    // custom <impl ast_processor>
    // end <impl ast_processor>
}

// --- module trait definitions ---

/// Trait to announce for logging/debug
trait AnnounceRender {
    /// Announce render of arg
    ///
    fn announce_render(&self) -> ();

    // custom <trait_announce_render>
    // end <trait_announce_render>
}

/// Trait to announce for logging/debug
trait AnnounceEval {
    /// Announce eval of arg
    ///
    fn announce_eval(&self) -> ();

    // custom <trait_announce_eval>
    // end <trait_announce_eval>
}

// --- module impl definitions ---

/// Implementation of trait `AnnounceRender` for type `Vec<Node>`
impl AnnounceRender for Vec<Node> {
    /// Announce render of arg
    ///
    #[inline]
    fn announce_render(&self) -> () {
        // custom <fn announce_render_vec_announce_render>

        info!("Render Vec<Node> ({})", self.len());

        // end <fn announce_render_vec_announce_render>
    }
}

/// Implementation of trait `AnnounceRender` for type `Forloop`
impl AnnounceRender for Forloop {
    /// Announce render of arg
    ///
    #[inline]
    fn announce_render(&self) -> () {
        // custom <fn announce_render_forloop_announce_render>

        info!("Render for_loop ({:?}, {}):", self.key, self.value);

        // end <fn announce_render_forloop_announce_render>
    }
}

/// Implementation of trait `AnnounceRender` for type `If`
impl AnnounceRender for If {
    /// Announce render of arg
    ///
    #[inline]
    fn announce_render(&self) -> () {
        // custom <fn announce_render_if_announce_render>
        info!(
            "Render if_node conditions({}), otherwise({}):",
            self.conditions.len(),
            self.otherwise.is_some()
        );
        // end <fn announce_render_if_announce_render>
    }
}

/// Implementation of trait `AnnounceRender` for type `Node`
impl AnnounceRender for Node {
    /// Announce render of arg
    ///
    #[inline]
    fn announce_render(&self) -> () {
        // custom <fn announce_render_node_announce_render>

        info!("Render node: {}", node_type(self));

        // end <fn announce_render_node_announce_render>
    }
}

/// Implementation of trait `AnnounceRender` for type `Block`
impl AnnounceRender for Block {
    /// Announce render of arg
    ///
    #[inline]
    fn announce_render(&self) -> () {
        // custom <fn announce_render_block_announce_render>

        info!("Render Block ({}) len({})", self.name, self.body.len());

        // end <fn announce_render_block_announce_render>
    }
}

/// Implementation of trait `AnnounceEval` for type `Expr`
impl AnnounceEval for Expr {
    /// Announce eval of arg
    ///
    #[inline]
    fn announce_eval(&self) -> () {
        // custom <fn announce_eval_expr_announce_eval>

        info!("Render Expr ({})", expr_val_type(&self.val));

        // end <fn announce_eval_expr_announce_eval>
    }
}

/// Implementation of trait `AnnounceEval` for type `FunctionCall`
impl AnnounceEval for FunctionCall {
    /// Announce eval of arg
    ///
    #[inline]
    fn announce_eval(&self) -> () {
        // custom <fn announce_eval_function_call_announce_eval>

        info!(
            "Render FnCall `{}({:?})`",
            self.name,
            self.args
                .keys()
                .map(|k| k.clone())
                .collect::<Vec<String>>()
                .join(", ")
        );

        // end <fn announce_eval_function_call_announce_eval>
    }
}

/// Implementation of trait `AnnounceEval` for type `MacroCall`
impl AnnounceEval for MacroCall {
    /// Announce eval of arg
    ///
    #[inline]
    fn announce_eval(&self) -> () {
        // custom <fn announce_eval_macro_call_announce_eval>

        info!(
            "Render Macro `{}::{}({})`",
            self.namespace,
            self.name,
            self.args
                .keys()
                .map(|k| k.clone())
                .collect::<Vec<String>>()
                .join(", ")
        );

        // end <fn announce_eval_macro_call_announce_eval>
    }
}

/// Implementation of trait `AnnounceEval` for type `Set`
impl AnnounceEval for Set {
    /// Announce eval of arg
    ///
    #[inline]
    fn announce_eval(&self) -> () {
        // custom <fn announce_eval_set_announce_eval>

        info!(
            "Render Set `{} = Expr({})`",
            self.key,
            expr_val_type(&self.value.val)
        );

        // end <fn announce_eval_set_announce_eval>
    }
}

/// Implementation of trait `AnnounceEval` for type `Test`
impl AnnounceEval for Test {
    /// Announce eval of arg
    ///
    #[inline]
    fn announce_eval(&self) -> () {
        // custom <fn announce_eval_test_announce_eval>

        info!("Render Test `{} is Expr({})`", self.ident, self.name);

        // end <fn announce_eval_test_announce_eval>
    }
}

// --- module function definitions ---

/// Returns text representation of node
///
///  * `node` - Node in question
///  * _return_ - Text representation of node
///
fn node_type(node: &Node) -> String {
    // custom <fn node_type>

    match node {
        Node::Super => "Super".into(),

        /// Some actual text
        Node::Text(s) => format!("Text of len({})", s.len()),

        /// A `{{ }}` block
        Node::VariableBlock(e) => format!("Variable Block"),

        /// A `{% macro hello() %}...{% endmacro %}`
        Node::MacroDefinition(_, macro_definition, _) => format!("Macro Definition"),

        /// The `{% extends "blabla.html" %}` node, contains the template name
        Node::Extends(_, s) => format!("Extends {}", s),

        /// The `{% include "blabla.html" %}` node, contains the template name
        Node::Include(_, s) => format!("Include {}", s),

        /// The `{% import "macros.html" as macros %}`
        Node::ImportMacro(_, file, namespace) => format!("Import ({}) as ({})", file, namespace),

        /// The `{% set val = something %}` tag
        Node::Set(_, set) => format!("Set"),

        /// The text between `{% raw %}` and `{% endraw %}`
        Node::Raw(_, s, _) => format!("Raw len({})", s.len()),

        /// A filter section node `{{ filter name(param="value") }} content {{ endfilter }}`
        Node::FilterSection(_, filter_section, _) => format!("Filter"),

        /// A `{% block name %}...{% endblock %}`
        Node::Block(_, block, _) => format!("Block"),

        /// A `{% for i in items %}...{% endfor %}`
        Node::Forloop(_, for_loop, _) => format!("Forloop"),

        /// A if/elif/else block, WS for the if/elif/else is directly in the struct
        Node::If(if_node, _) => format!("If"),

        /// The `{% break %}` tag
        Node::Break(_) => "break".into(),

        /// The `{% continue %}` tag
        Node::Continue(_) => "continue".into(),
    }
    // end <fn node_type>
}

/// Returns text representation of type
///
///  * `expr_val` - `ExprVal` in question
///  * _return_ - Text representation of node
///
fn expr_val_type(expr_val: &ExprVal) -> String {
    // custom <fn expr_val_type>

    match expr_val {
        ExprVal::String(s) => format!("Str({})", s.len()),
        ExprVal::Int(i) => format!("Int({})", i),
        ExprVal::Float(f) => format!("F64({})", f),
        ExprVal::Bool(b) => format!("Bool({})", b),
        ExprVal::Ident(i) => format!("Ident({})", i),
        ExprVal::Math(me) => format!("Math(lhs {:?} rhs)", me.operator),
        ExprVal::Logic(logic_expr) => format!("Logic({})", "TODO"),
        ExprVal::Test(test) => format!("Test({})", "TODO"),
        ExprVal::MacroCall(macro_call) => {
            format!("MacroCall({}:{})", macro_call.namespace, macro_call.name)
        }
        ExprVal::FunctionCall(function_call) => format!("Fn(TODO)"),
        // A vec of Expr, not ExprVal since filters are allowed
        // on values inside arrays
        ExprVal::Array(vec) => format!("Arr({})", vec.len()),
    }

    // end <fn expr_val_type>
}

// custom <module ast_processor ModuleBottom>

/// Implementation of trait `AnnounceRender` for type `[Node]`
impl AnnounceRender for [Node] {
    /// Announce render of arg
    ///
    #[inline]
    fn announce_render(&self) -> () {
        // custom <fn announce_render_node_announce_render>

        info!("Render [Node] ({})", self.len());

        // end <fn announce_render_node_announce_render>
    }
}

// end <module ast_processor ModuleBottom>
