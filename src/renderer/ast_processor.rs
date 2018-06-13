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
use renderer::path_processor::{process_path, Accessor};
use renderer::ref_or_owned::RefOrOwned;
use renderer::tera_macro::MacroCollection;
use serde_json::map::Map as JsonMap;
use serde_json::{to_string_pretty, to_value, Number, Value};
use std::collections::HashMap;
use template::Template;
use tera::Tera;
use utils::escape_html;

// --- module type aliases ---

type RenderResult = Result<String>;
type RefValue<'a> = RefOrOwned<'a, Value>;
type EvalResult<'a> = Result<RefValue<'a>>;
type LookupResult<'a> = Result<RefValue<'a>>;

// --- module statics ---

/// Special string indicating request to dump context
static MAGICAL_DUMP_VAR: &'static str = "__tera_context";

// --- module struct definitions ---

/// Processes the ast and renders the output
pub struct AstProcessor<'a> {
    /// The template to render
    template: &'a Template,
    /// Root template of template to render - contains ast to render
    template_root: &'a Template,
    /// The tera object with template details
    tera: &'a Tera,
    /// The call stack for processing
    call_stack: CallStack<'a>,
    /// The macro details
    macro_collection: MacroCollection<'a>,
    /// If set rendering should be escaped
    should_escape: bool,
    /// Used when super() is used in a block, to know where we are in our stack of
    /// definitions and for which block
    /// Vec<(block name, tpl_name, level)>
    ///
    blocks: Vec<(&'a str, &'a str, usize)>,
}
/// Implementation for type `AstProcessor`.
impl<'a> AstProcessor<'a> {
    /// Create a new `AstProcessor`
    ///
    ///  * `template` - The template to render
    ///  * `tera` - The tera object with template details
    ///  * `call_stack` - The call stack
    ///  * `macro_collection` - The macro details
    ///  * `should_escape` - If template should be escaped
    ///  * _return_ - Created `AstProcessor`
    ///
    pub fn new(
        template: &'a Template,
        tera: &'a Tera,
        call_stack: CallStack<'a>,
        macro_collection: MacroCollection<'a>,
        should_escape: bool,
    ) -> AstProcessor<'a> {
        let template_root = last_parent(tera, template).unwrap_or(template);

        AstProcessor {
            template,
            template_root,
            tera,
            call_stack,
            macro_collection,
            should_escape,
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
    fn take_macro_collection(self: &mut Self) -> MacroCollection<'a> {
        self.macro_collection.take_macro_collection()
    }

    /// Looks up identifier and returns its value
    ///
    ///  * `key` - Key to look up
    ///  * _return_ - Value if found
    ///
    fn lookup_ident(&self, key: &str) -> LookupResult<'a> {
        out!("lookup_ident: {}", key);

        // Magical variable that just dumps the context
        if key == MAGICAL_DUMP_VAR {
            // Unwraps are safe since we are dealing with things that are already Value
            return Ok(RefOrOwned::from_owned(
                to_value(
                    to_string_pretty(&self.call_stack.current_context_cloned().take()).unwrap(),
                ).unwrap(),
            ));
        }

        process_path(key, &self.call_stack)
    }

    /// Walk the ast and render
    ///
    ///  * `ast` - `ast` to render
    ///  * _return_ - Resulting `ast` rendering
    ///
    pub fn render_ast(self: &mut Self) -> RenderResult {
        self.template_root.ast.announce_render();

        let mut output = String::new();

        for node in self.template_root.ast.iter() {
            match self.render_node(node) {
                Ok(rendered) => {
                    output.push_str(&rendered);
                }
                Err(e) => bail!(format!(
                    "Failed to render `{}` - error location:\n{}{}",
                    self.call_stack.active_template().name,
                    self.call_stack.error_location(),
                    self.block_location()
                )),
            }
        }

        Ok(output)
    }

    /// If rendering a `block` determines location
    ///
    fn block_location(&self) -> String {
        if let Some(block_location) = self.blocks.last() {
            let parents = &self.call_stack.top_frame().active_template.parents;
            let num_parents = parents.len();
            let super_index = block_location.2;
            let offending_template = if let Some(parent) = parents.get(super_index) {
                parent
            } else {
                block_location.1
            };

            format!(
                "Rendering block `{}` in template `{}` with parent chain: {:?}",
                block_location.0, offending_template, parents
            )
        } else {
            String::new()
        }
    }

    /// Render for body
    ///
    ///  * `body` - `body` to render
    ///  * _return_ - Resulting `body` rendering
    ///
    fn render_body(self: &mut Self, body: &'a [Node]) -> RenderResult {
        body.announce_render();
        let mut result = String::with_capacity(body.len() * 16);

        for node in body {
            result.push_str(&self.render_node(node)?);
            if self.call_stack.should_break_body() {
                break;
            }
        }

        Ok(result)
    }

    /// Render for for_loop
    ///
    ///  * `for_loop` - `for_loop` to render
    ///  * _return_ - Resulting `for_loop` rendering
    ///
    fn render_for_loop(self: &mut Self, for_loop: &'a Forloop) -> RenderResult {
        for_loop.announce_render();
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

            if self.call_stack.should_break_for_loop() {
                break;
            }

            self.call_stack.increment_for_loop();
        }

        self.call_stack.pop_frame();

        Ok(output)
    }

    /// Render for if_node
    ///
    ///  * `if_node` - `if_node` to render
    ///  * _return_ - Resulting `if_node` rendering
    ///
    fn render_if_node(self: &mut Self, if_node: &'a If) -> RenderResult {
        if_node.announce_render();
        for &(_, ref expr, ref body) in &if_node.conditions {
            if self.eval_as_bool(expr)? {
                return self.render_body(body);
            }
        }

        if let Some((_, ref body)) = if_node.otherwise {
            return self.render_body(body);
        }

        Ok(String::new())
    }

    /// Render for node
    ///
    ///  * `node` - `node` to render
    ///  * _return_ - Resulting `node` rendering
    ///
    fn render_node(self: &mut Self, node: &'a Node) -> RenderResult {
        node.announce_render();
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
            Node::Super => self.do_super()?,
            Node::Include(_, ref tpl_name) => {
                let template = self.tera.get_template(tpl_name)?;
                self.macro_collection
                    .add_macros_from_template(self.tera, template);
                self.call_stack.push_include_frame(tpl_name, template);
                let result = self.render_body(&template.ast)?;
                self.call_stack.pop_frame();
                result
            }
            _ => unreachable!("render_node -> unexpected node: {:?}", node),
        };

        Ok(output)
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
    fn render_block(self: &mut Self, block: &'a Block, level: usize) -> RenderResult {
        block.announce_render();
        let level_template = match level {
            0 => self.call_stack.active_template(),
            _ => self
                .tera
                .get_template(&self.call_stack.active_template().parents[level - 1])
                .unwrap(),
        };

        let blocks_definitions = &level_template.blocks_definitions;

        // Can we find this one block in these definitions? If so render it
        if let Some(block_def) = blocks_definitions.get(&block.name) {
            let (ref tpl_name, Block { ref body, .. }) = block_def[0];
            self.blocks
                .push((&block.name[..], &level_template.name[..], level));
            return self.render_body(body);
        }

        // Do we have more parents to look through?
        if level + 1 <= self.call_stack.active_template().parents.len() {
            return self.render_block(block, level + 1);
        }

        // Nope, just render the body we got
        self.render_body(&block.body)
    }

    /// Render for expression
    ///
    ///  * `expr` - Render for expression
    ///  * _return_ - Resulting evaluation
    ///
    fn eval_expression(self: &mut Self, expr: &'a Expr) -> EvalResult<'a> {
        expr.announce_eval();

        let mut needs_escape = false;

        let mut res = match expr.val {
            ExprVal::Array(ref arr) => {
                let mut values = vec![];
                for v in arr {
                    values.push(self.eval_expression(v)?.take());
                }
                RefOrOwned::from_owned(Value::Array(values))
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

        // Checks if it's a string and we need to escape it (if the first filter is `safe` we don't)
        if self.should_escape
            && needs_escape
            && res.is_string()
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
    }

    /// Render for expression_safe
    ///
    ///  * `expr` - Render for expression_safe
    ///  * _return_ - Resulting evaluation
    ///
    fn eval_expression_safe(self: &mut Self, expr: &'a Expr) -> EvalResult<'a> {
        expr.announce_eval();
        let should_escape = self.should_escape;
        self.should_escape = false;
        let res = self.eval_expression(expr);
        self.should_escape = should_escape;
        res
    }

    /// Render for global_fn_call
    ///
    ///  * `function_call` - Render for global_fn_call
    ///  * _return_ - Resulting evaluation
    ///
    fn eval_global_fn_call(self: &mut Self, function_call: &'a FunctionCall) -> EvalResult<'a> {
        function_call.announce_eval();
        let global_fn = self.tera.get_global_function(&function_call.name)?;

        let mut args = HashMap::new();
        for (arg_name, expr) in &function_call.args {
            args.insert(
                arg_name.to_string(),
                self.eval_expression_safe(expr)?.get().clone(),
            );
        }

        Ok(RefOrOwned::from_owned(global_fn(args)?.take()))
    }

    /// Render for macro_call
    ///
    ///  * `macro_call` - Render for macro_call
    ///  * _return_ - Resulting evaluation
    ///
    fn eval_macro_call(self: &mut Self, macro_call: &'a MacroCall) -> Result<String> {
        macro_call.announce_eval();
        let mut active_template = self.call_stack.active_template();
        let active_template_name = if let Some(block) = self.blocks.last() {
            block.1
        } else {
            &active_template.name[..]
        };

        let (macro_template_name, macro_definition) = self.macro_collection.lookup_macro(
            active_template_name,
            &macro_call.namespace[..],
            &macro_call.name[..],
        )?;

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
            frame_context.insert(&arg_name[..], value);
        }

        out!(
            "Pushing macro {} with context {:?}",
            macro_call.name,
            frame_context.keys()
        );

        self.call_stack.push_macro_frame(
            &macro_call.name[..],
            frame_context,
            self.tera.get_template(macro_template_name)?,
        );

        let output = self.render_body(&macro_definition.body)?;

        self.call_stack.pop_frame();

        out!("Popped macro frame {}", &macro_call.name[..],);

        Ok(output)
    }

    /// Render for set
    ///
    ///  * `set` - Render for set
    ///  * _return_ - Resulting evaluation
    ///
    fn eval_set(self: &mut Self, set: &'a Set) -> Result<()> {
        set.announce_eval();
        let assigned_value = self.eval_expression_safe(&set.value)?;

        self.call_stack
            .add_assignment(&set.key[..], set.global, assigned_value);

        Ok(())
    }

    /// Render for test
    ///
    ///  * `test` - Render for test
    ///  * _return_ - Resulting evaluation
    ///
    fn eval_test(self: &mut Self, test: &'a Test) -> Result<bool> {
        test.announce_eval();
        let tester_fn = self.tera.get_tester(&test.name)?;

        let mut tester_args = vec![];
        for arg in &test.args {
            tester_args.push(self.eval_expression_safe(arg)?.get().clone());
        }

        let found = self
            .lookup_ident(&test.ident)
            .map(|found| found.get().clone())
            .ok();

        Ok(tester_fn(found, tester_args)?)
    }

    /// Evaluate filter on value
    ///
    ///  * `value` - Value to pass to filter
    ///  * `function_call` - Filter function
    ///  * _return_ - Resulting evaluation
    ///
    fn eval_filter(
        &mut self,
        value: &RefValue<'a>,
        function_call: &'a FunctionCall,
    ) -> EvalResult<'a> {
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
    }

    /// Evaluate expression as bool
    ///
    ///  * `bool_expr` - Boolean expression
    ///  * _return_ - Resulting evaluation
    ///
    fn eval_as_bool(&mut self, bool_expr: &'a Expr) -> Result<bool> {
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
            ExprVal::Ident(ref ident) => self
                .lookup_ident(ident)
                .map(|v| v.is_truthy())
                .unwrap_or(false),
            ExprVal::Math(_) | ExprVal::Int(_) | ExprVal::Float(_) => self
                .eval_as_number(&bool_expr.val)
                .map(|v| v != 0.0 && !v.is_nan())?,
            ExprVal::Test(ref test) => self.eval_test(test).unwrap_or(false),
            ExprVal::Bool(val) => val,
            ExprVal::String(ref string) => !string.is_empty(),
            _ => unreachable!("unimplemented logic operation for {:?}", bool_expr),
        };

        if bool_expr.negated {
            return Ok(!res);
        }

        Ok(res)
    }

    /// Evaluate expression value as number, monomorphing to f64
    ///
    ///  * `bool_expr` - Expression to evaluate as number normalized to f64
    ///  * _return_ - Resulting evaluation
    ///
    fn eval_expr_as_number(&mut self, bool_expr: &'a Expr) -> Result<f64> {
        if !bool_expr.filters.is_empty() {
            match self.eval_expression(bool_expr)?.get() {
                Value::Number(s) => Ok(s.as_f64().unwrap()),
                _ => bail!("Tried to do math with an expression not resulting in a number"),
            }
        } else {
            self.eval_as_number(&bool_expr.val)
        }
    }

    /// Evaluate expression as number, monomorphing to f64
    ///
    ///  * `expr_val` - Expression to evaluate as number normalized to f64
    ///  * _return_ - Resulting evaluation
    ///
    fn eval_as_number(&mut self, expr_val: &'a ExprVal) -> Result<f64> {
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
    }

    /// Only called while rendering a block.
    /// This will look up the block we are currently rendering and its level and try to render
    /// the block at level + n, where would be the next template in the hierarchy the block is present
    fn do_super(&mut self) -> Result<String> {
        let &(block_name, tpl_name, level) = self.blocks.last().unwrap();
        let mut next_level = level + 1;

        while next_level <= self.template.parents.len() {
            let blocks_definitions = &self
                .tera
                .get_template(&self.template.parents[next_level - 1])
                .unwrap()
                .blocks_definitions;

            if let Some(block_def) = blocks_definitions.get(block_name) {
                let (ref tpl_name, Block { ref body, .. }) = block_def[0];
                self.blocks.push((block_name, tpl_name, next_level));

                let res = self.render_body(body)?;
                self.blocks.pop();

                // Can't go any higher for that block anymore?
                if next_level >= self.template.parents.len() {
                    // then remove it from the stack, we're done with it
                    self.blocks.pop();
                }
                return Ok(res);
            } else {
                next_level += 1;
            }
        }

        bail!("Tried to use super() in the top level block")
    }
}

// --- module trait definitions ---

/// Trait to announce for logging/debug
trait AnnounceRender {
    /// Announce render of arg
    ///
    fn announce_render(&self) -> ();
}

/// Trait to announce for logging/debug
trait AnnounceEval {
    /// Announce eval of arg
    ///
    fn announce_eval(&self) -> ();
}

// --- module impl definitions ---

/// Implementation of trait `AnnounceRender` for type `Vec<Node>`
impl AnnounceRender for Vec<Node> {
    /// Announce render of arg
    ///
    #[inline]
    fn announce_render(&self) -> () {
        out!("Render Vec<Node> ({:?})", self);
    }
}

/// Implementation of trait `AnnounceRender` for type `Forloop`
impl AnnounceRender for Forloop {
    /// Announce render of arg
    ///
    #[inline]
    fn announce_render(&self) -> () {
        out!("Render for_loop ({:?}, {}):", self.key, self.value);
    }
}

/// Implementation of trait `AnnounceRender` for type `If`
impl AnnounceRender for If {
    /// Announce render of arg
    ///
    #[inline]
    fn announce_render(&self) -> () {
        out!(
            "Render if_node conditions({}), otherwise({}):",
            self.conditions.len(),
            self.otherwise.is_some()
        );
    }
}

/// Implementation of trait `AnnounceRender` for type `Node`
impl AnnounceRender for Node {
    /// Announce render of arg
    ///
    #[inline]
    fn announce_render(&self) -> () {
        out!("Render node: {} -> {:?}", node_type(self), self);
    }
}

/// Implementation of trait `AnnounceRender` for type `Block`
impl AnnounceRender for Block {
    /// Announce render of arg
    ///
    #[inline]
    fn announce_render(&self) -> () {
        out!(
            "Render Block (`{}`) len({}) -> {:?}",
            self.name,
            self.body.len(),
            self
        );
    }
}

/// Implementation of trait `AnnounceEval` for type `Expr`
impl AnnounceEval for Expr {
    /// Announce eval of arg
    ///
    #[inline]
    fn announce_eval(&self) -> () {
        out!("Render Expr ({})", expr_val_type(&self.val));
    }
}

/// Implementation of trait `AnnounceEval` for type `FunctionCall`
impl AnnounceEval for FunctionCall {
    /// Announce eval of arg
    ///
    #[inline]
    fn announce_eval(&self) -> () {
        out!(
            "Render FnCall `{}({:?})`",
            self.name,
            self.args
                .keys()
                .map(|k| k.clone())
                .collect::<Vec<String>>()
                .join(", ")
        );
    }
}

/// Implementation of trait `AnnounceEval` for type `MacroCall`
impl AnnounceEval for MacroCall {
    /// Announce eval of arg
    ///
    #[inline]
    fn announce_eval(&self) -> () {
        out!(
            "Render Macro `{}::{}({})`",
            self.namespace,
            self.name,
            self.args
                .keys()
                .map(|k| k.clone())
                .collect::<Vec<String>>()
                .join(", ")
        );
    }
}

/// Implementation of trait `AnnounceEval` for type `Set`
impl AnnounceEval for Set {
    /// Announce eval of arg
    ///
    #[inline]
    fn announce_eval(&self) -> () {
        out!(
            "Render Set `{} = Expr({})`",
            self.key,
            expr_val_type(&self.value.val)
        );
    }
}

/// Implementation of trait `AnnounceEval` for type `Test`
impl AnnounceEval for Test {
    /// Announce eval of arg
    ///
    #[inline]
    fn announce_eval(&self) -> () {
        out!("Render Test `{} is Expr({})`", self.ident, self.name);
    }
}

// --- module function definitions ---

/// Returns text representation of node
///
///  * `node` - Node in question
///  * _return_ - Text representation of node
///
fn node_type(node: &Node) -> String {
    match node {
        Node::Super => "Super".into(),

        /// Some actual text
        Node::Text(s) => format!("Text of len({}) -> `{}`", s.len(), s),

        /// A `{{ }}` block
        Node::VariableBlock(e) => format!("Variable Block `{:?}`", e),

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
}

/// Returns text representation of type
///
///  * `expr_val` - `ExprVal` in question
///  * _return_ - Text representation of node
///
fn expr_val_type(expr_val: &ExprVal) -> String {
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
}

/// Implementation of trait `AnnounceRender` for type `[Node]`
impl AnnounceRender for [Node] {
    /// Announce render of arg
    ///
    #[inline]
    fn announce_render(&self) -> () {
        out!("Render [Node] ({:?})", self);
    }
}

/// Get last parent template
///
///  * `tera` - Tera that contains templates
///  * `template` - Template to find last template of
///  * _return_ - Last parent of template or `None`
///
#[inline]
pub fn last_parent<'a>(tera: &'a Tera, template: &'a Template) -> Option<&'a Template> {
    template
        .parents
        .last()
        .map(|parent| tera.get_template(parent).unwrap())
}
