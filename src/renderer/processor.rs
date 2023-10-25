use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Write;

use serde_json::{to_string_pretty, to_value, Number, Value};

use crate::context::{ValueRender, ValueTruthy};
use crate::errors::{Error, Result};
use crate::parser::ast::*;
use crate::renderer::call_stack::CallStack;
use crate::renderer::for_loop::ForLoop;
use crate::renderer::macros::MacroCollection;
use crate::renderer::square_brackets::pull_out_square_bracket;
use crate::renderer::stack_frame::{FrameContext, FrameType, Val};
use crate::template::Template;
use crate::tera::Tera;
use crate::utils::render_to_string;
use crate::Context;

/// Special string indicating request to dump context
static MAGICAL_DUMP_VAR: &str = "__tera_context";

/// This will convert a Tera variable to a json pointer if it is possible by replacing
/// the index with their evaluated stringified value
fn evaluate_sub_variables(key: &str, call_stack: &CallStack) -> Result<String> {
    let sub_vars_to_calc = pull_out_square_bracket(key);
    let mut new_key = key.to_string();

    for sub_var in &sub_vars_to_calc {
        // Translate from variable name to variable value
        match process_path(sub_var.as_ref(), call_stack) {
            Err(e) => {
                return Err(Error::msg(format!(
                    "Variable {} can not be evaluated because: {}",
                    key, e
                )));
            }
            Ok(post_var) => {
                let post_var_as_str = match *post_var {
                    Value::String(ref s) => format!(r#""{}""#, s),
                    Value::Number(ref n) => n.to_string(),
                    _ => {
                        return Err(Error::msg(format!(
                            "Only variables evaluating to String or Number can be used as \
                             index (`{}` of `{}`)",
                            sub_var, key,
                        )));
                    }
                };

                // Rebuild the original key String replacing variable name with value
                let nk = new_key.clone();
                let divider = "[".to_string() + sub_var + "]";
                let mut the_parts = nk.splitn(2, divider.as_str());

                new_key = the_parts.next().unwrap().to_string()
                    + "."
                    + post_var_as_str.as_ref()
                    + the_parts.next().unwrap_or("");
            }
        }
    }

    Ok(new_key
        .replace('/', "~1") // https://tools.ietf.org/html/rfc6901#section-3
        .replace("['", ".\"")
        .replace("[\"", ".\"")
        .replace('[', ".")
        .replace("']", "\"")
        .replace("\"]", "\"")
        .replace(']', ""))
}

fn process_path<'a>(path: &str, call_stack: &CallStack<'a>) -> Result<Val<'a>> {
    if !path.contains('[') {
        match call_stack.lookup(path) {
            Some(v) => Ok(v),
            None => Err(Error::msg(format!(
                "Variable `{}` not found in context while rendering '{}'",
                path,
                call_stack.active_template().name
            ))),
        }
    } else {
        let full_path = evaluate_sub_variables(path, call_stack)?;

        match call_stack.lookup(&full_path) {
            Some(v) => Ok(v),
            None => Err(Error::msg(format!(
                "Variable `{}` not found in context while rendering '{}': \
                 the evaluated version was `{}`. Maybe the index is out of bounds?",
                path,
                call_stack.active_template().name,
                full_path,
            ))),
        }
    }
}

/// Processes the ast and renders the output
pub struct Processor<'a> {
    /// The template we're trying to render
    template: &'a Template,
    /// Root template of template to render - contains ast to use for rendering
    /// Can be the same as `template` if a template has no inheritance
    template_root: &'a Template,
    /// The Tera object with template details
    tera: &'a Tera,
    /// The call stack for processing
    call_stack: CallStack<'a>,
    /// The macros organised by template and namespaces
    macros: MacroCollection<'a>,
    /// If set, rendering should be escaped
    should_escape: bool,
    /// Used when super() is used in a block, to know where we are in our stack of
    /// definitions and for which block
    /// Vec<(block name, tpl_name, level)>
    blocks: Vec<(&'a str, &'a str, usize)>,
}

impl<'a> Processor<'a> {
    /// Create a new `Processor` that will do the rendering
    pub fn new(
        template: &'a Template,
        tera: &'a Tera,
        context: &'a Context,
        should_escape: bool,
    ) -> Self {
        // Gets the root template if we are rendering something with inheritance or just return
        // the template we're dealing with otherwise
        let template_root = template
            .parents
            .last()
            .map(|parent| tera.get_template(parent).unwrap())
            .unwrap_or(template);

        let call_stack = CallStack::new(context, template);

        Processor {
            template,
            template_root,
            tera,
            call_stack,
            macros: MacroCollection::from_original_template(template, tera),
            should_escape,
            blocks: Vec::new(),
        }
    }

    fn render_body(&mut self, body: &'a [Node], write: &mut impl Write) -> Result<()> {
        for n in body {
            self.render_node(n, write)?;

            if self.call_stack.should_break_body() {
                break;
            }
        }

        Ok(())
    }

    fn render_for_loop(&mut self, for_loop: &'a Forloop, write: &mut impl Write) -> Result<()> {
        let container_name = match for_loop.container.val {
            ExprVal::Ident(ref ident) => ident,
            ExprVal::FunctionCall(FunctionCall { ref name, .. }) => name,
            ExprVal::Array(_) => "an array literal",
            _ => return Err(Error::msg(format!(
                "Forloop containers have to be an ident or a function call (tried to iterate on '{:?}')",
                for_loop.container.val,
            ))),
        };

        let for_loop_name = &for_loop.value;
        let for_loop_body = &for_loop.body;
        let for_loop_empty_body = &for_loop.empty_body;

        let container_val = self.safe_eval_expression(&for_loop.container)?;

        let for_loop = match *container_val {
            Value::Array(_) => {
                if for_loop.key.is_some() {
                    return Err(Error::msg(format!(
                        "Tried to iterate using key value on variable `{}`, but it isn't an object/map",
                        container_name,
                    )));
                }
                ForLoop::from_array(&for_loop.value, container_val)
            }
            Value::String(_) => {
                if for_loop.key.is_some() {
                    return Err(Error::msg(format!(
                        "Tried to iterate using key value on variable `{}`, but it isn't an object/map",
                        container_name,
                    )));
                }
                ForLoop::from_string(&for_loop.value, container_val)
            }
            Value::Object(_) => {
                if for_loop.key.is_none() {
                    return Err(Error::msg(format!(
                        "Tried to iterate using key value on variable `{}`, but it is missing a key",
                        container_name,
                    )));
                }
                match container_val {
                    Cow::Borrowed(c) => {
                        ForLoop::from_object(for_loop.key.as_ref().unwrap(), &for_loop.value, c)
                    }
                    Cow::Owned(c) => ForLoop::from_object_owned(
                        for_loop.key.as_ref().unwrap(),
                        &for_loop.value,
                        c,
                    ),
                }
            }
            _ => {
                return Err(Error::msg(format!(
                    "Tried to iterate on a container (`{}`) that has a unsupported type",
                    container_name,
                )));
            }
        };

        let len = for_loop.len();
        match (len, for_loop_empty_body) {
            (0, Some(empty_body)) => self.render_body(empty_body, write),
            (0, _) => Ok(()),
            (_, _) => {
                self.call_stack.push_for_loop_frame(for_loop_name, for_loop);

                for _ in 0..len {
                    self.render_body(for_loop_body, write)?;

                    if self.call_stack.should_break_for_loop() {
                        break;
                    }

                    self.call_stack.increment_for_loop()?;
                }

                self.call_stack.pop();

                Ok(())
            }
        }
    }

    fn render_if_node(&mut self, if_node: &'a If, write: &mut impl Write) -> Result<()> {
        for (_, expr, body) in &if_node.conditions {
            if self.eval_as_bool(expr)? {
                return self.render_body(body, write);
            }
        }

        if let Some((_, ref body)) = if_node.otherwise {
            return self.render_body(body, write);
        }

        Ok(())
    }

    /// The way inheritance work is that the top parent will be rendered by the renderer so for blocks
    /// we want to look from the bottom (`level = 0`, the template the user is actually rendering)
    /// to the top (the base template).
    fn render_block(
        &mut self,
        block: &'a Block,
        level: usize,
        write: &mut impl Write,
    ) -> Result<()> {
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
            let (_, Block { ref body, .. }) = block_def[0];
            self.blocks.push((&block.name[..], &level_template.name[..], level));
            return self.render_body(body, write);
        }

        // Do we have more parents to look through?
        if level < self.call_stack.active_template().parents.len() {
            return self.render_block(block, level + 1, write);
        }

        // Nope, just render the body we got
        self.render_body(&block.body, write)
    }

    fn get_default_value(&mut self, expr: &'a Expr) -> Result<Val<'a>> {
        if let Some(default_expr) = expr.filters[0].args.get("value") {
            self.eval_expression(default_expr)
        } else {
            Err(Error::msg("The `default` filter requires a `value` argument."))
        }
    }

    fn eval_in_condition(&mut self, in_cond: &'a In) -> Result<bool> {
        let lhs = self.safe_eval_expression(&in_cond.lhs)?;
        let rhs = self.safe_eval_expression(&in_cond.rhs)?;

        let present = match *rhs {
            Value::Array(ref v) => v.contains(&lhs),
            Value::String(ref s) => match *lhs {
                Value::String(ref s2) => s.contains(s2),
                _ => {
                    return Err(Error::msg(format!(
                        "Tried to check if {:?} is in a string, but it isn't a string",
                        lhs
                    )))
                }
            },
            Value::Object(ref map) => match *lhs {
                Value::String(ref s2) => map.contains_key(s2),
                _ => {
                    return Err(Error::msg(format!(
                        "Tried to check if {:?} is in a object, but it isn't a string",
                        lhs
                    )))
                }
            },
            _ => {
                return Err(Error::msg(
                    "The `in` operator only supports strings, arrays and objects.",
                ))
            }
        };

        Ok(if in_cond.negated { !present } else { present })
    }

    fn eval_expression(&mut self, expr: &'a Expr) -> Result<Val<'a>> {
        let mut needs_escape = false;

        let mut res = match expr.val {
            ExprVal::Array(ref arr) => {
                let mut values = vec![];
                for v in arr {
                    values.push(self.eval_expression(v)?.into_owned());
                }
                Cow::Owned(Value::Array(values))
            }
            ExprVal::In(ref in_cond) => Cow::Owned(Value::Bool(self.eval_in_condition(in_cond)?)),
            ExprVal::String(ref val) => {
                needs_escape = true;
                Cow::Owned(Value::String(val.to_string()))
            }
            ExprVal::StringConcat(ref str_concat) => {
                let mut res = String::new();
                for s in &str_concat.values {
                    match *s {
                        ExprVal::String(ref v) => res.push_str(v),
                        ExprVal::Int(ref v) => res.push_str(&format!("{}", v)),
                        ExprVal::Float(ref v) => res.push_str(&format!("{}", v)),
                        ExprVal::Ident(ref i) => match *self.lookup_ident(i)? {
                            Value::String(ref v) => res.push_str(v),
                            Value::Number(ref v) => res.push_str(&v.to_string()),
                            _ => return Err(Error::msg(format!(
                                "Tried to concat a value that is not a string or a number from ident {}",
                                i
                            ))),
                        },
                        ExprVal::FunctionCall(ref fn_call) => match *self.eval_tera_fn_call(fn_call, &mut needs_escape)? {
                            Value::String(ref v) => res.push_str(v),
                            Value::Number(ref v) => res.push_str(&v.to_string()),
                            _ => return Err(Error::msg(format!(
                                "Tried to concat a value that is not a string or a number from function call {}",
                                fn_call.name
                            ))),
                        },
                        _ => unreachable!(),
                    };
                }

                Cow::Owned(Value::String(res))
            }
            ExprVal::Int(val) => Cow::Owned(Value::Number(val.into())),
            ExprVal::Float(val) => Cow::Owned(Value::Number(Number::from_f64(val).unwrap())),
            ExprVal::Bool(val) => Cow::Owned(Value::Bool(val)),
            ExprVal::Ident(ref ident) => {
                needs_escape = ident != MAGICAL_DUMP_VAR;
                // Negated idents are special cased as `not undefined_ident` should not
                // error but instead be falsy values
                match self.lookup_ident(ident) {
                    Ok(val) => {
                        if val.is_null() && expr.has_default_filter() {
                            self.get_default_value(expr)?
                        } else {
                            val
                        }
                    }
                    Err(e) => {
                        if expr.has_default_filter() {
                            self.get_default_value(expr)?
                        } else {
                            if !expr.negated {
                                return Err(e);
                            }
                            // A negative undefined ident is !false so truthy
                            return Ok(Cow::Owned(Value::Bool(true)));
                        }
                    }
                }
            }
            ExprVal::FunctionCall(ref fn_call) => {
                self.eval_tera_fn_call(fn_call, &mut needs_escape)?
            }
            ExprVal::MacroCall(ref macro_call) => {
                let val = render_to_string(
                    || format!("macro {}", macro_call.name),
                    |w| self.eval_macro_call(macro_call, w),
                )?;
                Cow::Owned(Value::String(val))
            }
            ExprVal::Test(ref test) => Cow::Owned(Value::Bool(self.eval_test(test)?)),
            ExprVal::Logic(_) => Cow::Owned(Value::Bool(self.eval_as_bool(expr)?)),
            ExprVal::Math(_) => match self.eval_as_number(&expr.val) {
                Ok(Some(n)) => Cow::Owned(Value::Number(n)),
                Ok(None) => Cow::Owned(Value::String("NaN".to_owned())),
                Err(e) => return Err(Error::msg(e)),
            },
        };

        for filter in &expr.filters {
            if filter.name == "safe" || filter.name == "default" {
                continue;
            }
            res = self.eval_filter(&res, filter, &mut needs_escape)?;
        }

        // Lastly, we need to check if the expression is negated, thus turning it into a bool
        if expr.negated {
            return Ok(Cow::Owned(Value::Bool(!res.is_truthy())));
        }

        // Checks if it's a string and we need to escape it (if the last filter is `safe` we don't)
        if self.should_escape && needs_escape && res.is_string() && !expr.is_marked_safe() {
            res = Cow::Owned(
                to_value(self.tera.get_escape_fn()(res.as_str().unwrap())).map_err(Error::json)?,
            );
        }

        Ok(res)
    }

    /// Render an expression and never escape its result
    fn safe_eval_expression(&mut self, expr: &'a Expr) -> Result<Val<'a>> {
        let should_escape = self.should_escape;
        self.should_escape = false;
        let res = self.eval_expression(expr);
        self.should_escape = should_escape;
        res
    }

    /// Evaluate a set tag and add the value to the right context
    fn eval_set(&mut self, set: &'a Set) -> Result<()> {
        let assigned_value = self.safe_eval_expression(&set.value)?;
        self.call_stack.add_assignment(&set.key[..], set.global, assigned_value);
        Ok(())
    }

    fn eval_test(&mut self, test: &'a Test) -> Result<bool> {
        let tester_fn = self.tera.get_tester(&test.name)?;
        let err_wrap = |e| Error::call_test(&test.name, e);

        let mut tester_args = vec![];
        for arg in &test.args {
            tester_args
                .push(self.safe_eval_expression(arg).map_err(err_wrap)?.clone().into_owned());
        }

        let found = self.lookup_ident(&test.ident).map(|found| found.clone().into_owned()).ok();

        let result = tester_fn.test(found.as_ref(), &tester_args).map_err(err_wrap)?;
        if test.negated {
            Ok(!result)
        } else {
            Ok(result)
        }
    }

    fn eval_tera_fn_call(
        &mut self,
        function_call: &'a FunctionCall,
        needs_escape: &mut bool,
    ) -> Result<Val<'a>> {
        let tera_fn = self.tera.get_function(&function_call.name)?;
        *needs_escape = !tera_fn.is_safe();

        let err_wrap = |e| Error::call_function(&function_call.name, e);

        let mut args = HashMap::with_capacity(function_call.args.len());
        for (arg_name, expr) in &function_call.args {
            args.insert(
                arg_name.to_string(),
                self.safe_eval_expression(expr).map_err(err_wrap)?.clone().into_owned(),
            );
        }

        Ok(Cow::Owned(tera_fn.call(&args).map_err(err_wrap)?))
    }

    fn eval_macro_call(&mut self, macro_call: &'a MacroCall, write: &mut impl Write) -> Result<()> {
        let active_template_name = if let Some(block) = self.blocks.last() {
            block.1
        } else if self.template.name != self.template_root.name {
            &self.template_root.name
        } else {
            &self.call_stack.active_template().name
        };

        let (macro_template_name, macro_definition) = self.macros.lookup_macro(
            active_template_name,
            &macro_call.namespace[..],
            &macro_call.name[..],
        )?;

        let mut frame_context = FrameContext::with_capacity(macro_definition.args.len());

        // First the default arguments
        for (arg_name, default_value) in &macro_definition.args {
            let value = match macro_call.args.get(arg_name) {
                Some(val) => self.safe_eval_expression(val)?,
                None => match *default_value {
                    Some(ref val) => self.safe_eval_expression(val)?,
                    None => {
                        return Err(Error::msg(format!(
                            "Macro `{}` is missing the argument `{}`",
                            macro_call.name, arg_name
                        )));
                    }
                },
            };
            frame_context.insert(arg_name, value);
        }

        self.call_stack.push_macro_frame(
            &macro_call.namespace,
            &macro_call.name,
            frame_context,
            self.tera.get_template(macro_template_name)?,
        );

        self.render_body(&macro_definition.body, write)?;

        self.call_stack.pop();

        Ok(())
    }

    fn eval_filter(
        &mut self,
        value: &Val<'a>,
        fn_call: &'a FunctionCall,
        needs_escape: &mut bool,
    ) -> Result<Val<'a>> {
        let filter_fn = self.tera.get_filter(&fn_call.name)?;
        *needs_escape = !filter_fn.is_safe();

        let err_wrap = |e| Error::call_filter(&fn_call.name, e);

        let mut args = HashMap::with_capacity(fn_call.args.len());
        for (arg_name, expr) in &fn_call.args {
            args.insert(
                arg_name.to_string(),
                self.safe_eval_expression(expr).map_err(err_wrap)?.clone().into_owned(),
            );
        }

        Ok(Cow::Owned(filter_fn.filter(value, &args).map_err(err_wrap)?))
    }

    fn eval_as_bool(&mut self, bool_expr: &'a Expr) -> Result<bool> {
        let res = match bool_expr.val {
            ExprVal::Logic(LogicExpr { ref lhs, ref rhs, ref operator }) => {
                match *operator {
                    LogicOperator::Or => self.eval_as_bool(lhs)? || self.eval_as_bool(rhs)?,
                    LogicOperator::And => self.eval_as_bool(lhs)? && self.eval_as_bool(rhs)?,
                    LogicOperator::Gt
                    | LogicOperator::Gte
                    | LogicOperator::Lt
                    | LogicOperator::Lte => {
                        let l = self.eval_expr_as_number(lhs)?;
                        let r = self.eval_expr_as_number(rhs)?;
                        let (ll, rr) = match (l, r) {
                            (Some(nl), Some(nr)) => (nl, nr),
                            _ => return Err(Error::msg("Comparison to NaN")),
                        };

                        match *operator {
                            LogicOperator::Gte => ll.as_f64().unwrap() >= rr.as_f64().unwrap(),
                            LogicOperator::Gt => ll.as_f64().unwrap() > rr.as_f64().unwrap(),
                            LogicOperator::Lte => ll.as_f64().unwrap() <= rr.as_f64().unwrap(),
                            LogicOperator::Lt => ll.as_f64().unwrap() < rr.as_f64().unwrap(),
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

                            lhs_val = Cow::Owned(Value::Number(
                                Number::from_f64(lhs_val.as_f64().unwrap()).unwrap(),
                            ));
                            rhs_val = Cow::Owned(Value::Number(
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
            ExprVal::Ident(_) => {
                let mut res = self
                    .eval_expression(bool_expr)
                    .unwrap_or(Cow::Owned(Value::Bool(false)))
                    .is_truthy();
                if bool_expr.negated {
                    res = !res;
                }
                res
            }
            ExprVal::Math(_) | ExprVal::Int(_) | ExprVal::Float(_) => {
                match self.eval_as_number(&bool_expr.val)? {
                    Some(n) => n.as_f64().unwrap() != 0.0,
                    None => false,
                }
            }
            ExprVal::In(ref in_cond) => self.eval_in_condition(in_cond)?,
            ExprVal::Test(ref test) => self.eval_test(test)?,
            ExprVal::Bool(val) => val,
            ExprVal::String(ref string) => !string.is_empty(),
            ExprVal::FunctionCall(ref fn_call) => {
                let v = self.eval_tera_fn_call(fn_call, &mut false)?;
                match v.as_bool() {
                    Some(val) => val,
                    None => {
                        return Err(Error::msg(format!(
                            "Function `{}` was used in a logic operation but is not returning a bool",
                            fn_call.name,
                        )));
                    }
                }
            }
            ExprVal::StringConcat(_) => {
                let res = self.eval_expression(bool_expr)?;
                !res.as_str().unwrap().is_empty()
            }
            ExprVal::MacroCall(ref macro_call) => {
                let mut buf = Vec::new();
                self.eval_macro_call(macro_call, &mut buf)?;
                !buf.is_empty()
            }
            _ => unreachable!("unimplemented logic operation for {:?}", bool_expr),
        };

        if bool_expr.negated {
            return Ok(!res);
        }

        Ok(res)
    }

    /// In some cases, we will have filters in lhs/rhs of a math expression
    /// `eval_as_number` only works on ExprVal rather than Expr
    fn eval_expr_as_number(&mut self, expr: &'a Expr) -> Result<Option<Number>> {
        if !expr.filters.is_empty() {
            match *self.eval_expression(expr)? {
                Value::Number(ref s) => Ok(Some(s.clone())),
                _ => {
                    Err(Error::msg("Tried to do math with an expression not resulting in a number"))
                }
            }
        } else {
            self.eval_as_number(&expr.val)
        }
    }

    /// Return the value of an expression as a number
    fn eval_as_number(&mut self, expr: &'a ExprVal) -> Result<Option<Number>> {
        let result = match *expr {
            ExprVal::Ident(ref ident) => {
                let v = &*self.lookup_ident(ident)?;
                if v.is_i64() {
                    Some(Number::from(v.as_i64().unwrap()))
                } else if v.is_u64() {
                    Some(Number::from(v.as_u64().unwrap()))
                } else if v.is_f64() {
                    Some(Number::from_f64(v.as_f64().unwrap()).unwrap())
                } else {
                    return Err(Error::msg(format!(
                        "Variable `{}` was used in a math operation but is not a number",
                        ident
                    )));
                }
            }
            ExprVal::Int(val) => Some(Number::from(val)),
            ExprVal::Float(val) => Some(Number::from_f64(val).unwrap()),
            ExprVal::Math(MathExpr { ref lhs, ref rhs, ref operator }) => {
                let (l, r) = match (self.eval_expr_as_number(lhs)?, self.eval_expr_as_number(rhs)?)
                {
                    (Some(l), Some(r)) => (l, r),
                    _ => return Ok(None),
                };

                match *operator {
                    MathOperator::Mul => {
                        if l.is_i64() && r.is_i64() {
                            let ll = l.as_i64().unwrap();
                            let rr = r.as_i64().unwrap();
                            let res = match ll.checked_mul(rr) {
                                Some(s) => s,
                                None => {
                                    return Err(Error::msg(format!(
                                        "{} x {} results in an out of bounds i64",
                                        ll, rr
                                    )));
                                }
                            };

                            Some(Number::from(res))
                        } else if l.is_u64() && r.is_u64() {
                            let ll = l.as_u64().unwrap();
                            let rr = r.as_u64().unwrap();
                            let res = match ll.checked_mul(rr) {
                                Some(s) => s,
                                None => {
                                    return Err(Error::msg(format!(
                                        "{} x {} results in an out of bounds u64",
                                        ll, rr
                                    )));
                                }
                            };
                            Some(Number::from(res))
                        } else {
                            let ll = l.as_f64().unwrap();
                            let rr = r.as_f64().unwrap();
                            Number::from_f64(ll * rr)
                        }
                    }
                    MathOperator::Div => {
                        let ll = l.as_f64().unwrap();
                        let rr = r.as_f64().unwrap();
                        let res = ll / rr;
                        if res.is_nan() {
                            None
                        } else if res.round() == res && res.is_finite() {
                            Some(Number::from(res as i64))
                        } else {
                            Number::from_f64(res)
                        }
                    }
                    MathOperator::Add => {
                        if l.is_i64() && r.is_i64() {
                            let ll = l.as_i64().unwrap();
                            let rr = r.as_i64().unwrap();
                            let res = match ll.checked_add(rr) {
                                Some(s) => s,
                                None => {
                                    return Err(Error::msg(format!(
                                        "{} + {} results in an out of bounds i64",
                                        ll, rr
                                    )));
                                }
                            };
                            Some(Number::from(res))
                        } else if l.is_u64() && r.is_u64() {
                            let ll = l.as_u64().unwrap();
                            let rr = r.as_u64().unwrap();
                            let res = match ll.checked_add(rr) {
                                Some(s) => s,
                                None => {
                                    return Err(Error::msg(format!(
                                        "{} + {} results in an out of bounds u64",
                                        ll, rr
                                    )));
                                }
                            };
                            Some(Number::from(res))
                        } else {
                            let ll = l.as_f64().unwrap();
                            let rr = r.as_f64().unwrap();
                            Some(Number::from_f64(ll + rr).unwrap())
                        }
                    }
                    MathOperator::Sub => {
                        if l.is_i64() && r.is_i64() {
                            let ll = l.as_i64().unwrap();
                            let rr = r.as_i64().unwrap();
                            let res = match ll.checked_sub(rr) {
                                Some(s) => s,
                                None => {
                                    return Err(Error::msg(format!(
                                        "{} - {} results in an out of bounds i64",
                                        ll, rr
                                    )));
                                }
                            };
                            Some(Number::from(res))
                        } else if l.is_u64() && r.is_u64() {
                            let ll = l.as_u64().unwrap();
                            let rr = r.as_u64().unwrap();
                            let res = match ll.checked_sub(rr) {
                                Some(s) => s,
                                None => {
                                    return Err(Error::msg(format!(
                                        "{} - {} results in an out of bounds u64",
                                        ll, rr
                                    )));
                                }
                            };
                            Some(Number::from(res))
                        } else {
                            let ll = l.as_f64().unwrap();
                            let rr = r.as_f64().unwrap();
                            Some(Number::from_f64(ll - rr).unwrap())
                        }
                    }
                    MathOperator::Modulo => {
                        if l.is_i64() && r.is_i64() {
                            let ll = l.as_i64().unwrap();
                            let rr = r.as_i64().unwrap();
                            if rr == 0 {
                                return Err(Error::msg(format!(
                                    "Tried to do a modulo by zero: {:?}/{:?}",
                                    lhs, rhs
                                )));
                            }
                            Some(Number::from(ll % rr))
                        } else if l.is_u64() && r.is_u64() {
                            let ll = l.as_u64().unwrap();
                            let rr = r.as_u64().unwrap();
                            if rr == 0 {
                                return Err(Error::msg(format!(
                                    "Tried to do a modulo by zero: {:?}/{:?}",
                                    lhs, rhs
                                )));
                            }
                            Some(Number::from(ll % rr))
                        } else {
                            let ll = l.as_f64().unwrap();
                            let rr = r.as_f64().unwrap();
                            Number::from_f64(ll % rr)
                        }
                    }
                }
            }
            ExprVal::FunctionCall(ref fn_call) => {
                let v = self.eval_tera_fn_call(fn_call, &mut false)?;
                if v.is_i64() {
                    Some(Number::from(v.as_i64().unwrap()))
                } else if v.is_u64() {
                    Some(Number::from(v.as_u64().unwrap()))
                } else if v.is_f64() {
                    Some(Number::from_f64(v.as_f64().unwrap()).unwrap())
                } else {
                    return Err(Error::msg(format!(
                        "Function `{}` was used in a math operation but is not returning a number",
                        fn_call.name
                    )));
                }
            }
            ExprVal::String(ref val) => {
                return Err(Error::msg(format!("Tried to do math with a string: `{}`", val)));
            }
            ExprVal::Bool(val) => {
                return Err(Error::msg(format!("Tried to do math with a boolean: `{}`", val)));
            }
            ExprVal::StringConcat(ref val) => {
                return Err(Error::msg(format!(
                    "Tried to do math with a string concatenation: {}",
                    val.to_template_string()
                )));
            }
            ExprVal::Test(ref test) => {
                return Err(Error::msg(format!("Tried to do math with a test: {}", test.name)));
            }
            _ => unreachable!("unimplemented math expression for {:?}", expr),
        };

        Ok(result)
    }

    /// Only called while rendering a block.
    /// This will look up the block we are currently rendering and its level and try to render
    /// the block at level + n, where would be the next template in the hierarchy the block is present
    fn do_super(&mut self, write: &mut impl Write) -> Result<()> {
        let &(block_name, _, level) = self.blocks.last().unwrap();
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

                self.render_body(body, write)?;
                self.blocks.pop();

                // Can't go any higher for that block anymore?
                if next_level >= self.template.parents.len() {
                    // then remove it from the stack, we're done with it
                    self.blocks.pop();
                }
                return Ok(());
            } else {
                next_level += 1;
            }
        }

        Err(Error::msg("Tried to use super() in the top level block"))
    }

    /// Looks up identifier and returns its value
    fn lookup_ident(&self, key: &str) -> Result<Val<'a>> {
        // Magical variable that just dumps the context
        if key == MAGICAL_DUMP_VAR {
            // Unwraps are safe since we are dealing with things that are already Value
            return Ok(Cow::Owned(
                to_value(
                    to_string_pretty(&self.call_stack.current_context_cloned().take()).unwrap(),
                )
                .unwrap(),
            ));
        }

        process_path(key, &self.call_stack)
    }

    /// Process the given node, appending the string result to the buffer
    /// if it is possible
    fn render_node(&mut self, node: &'a Node, write: &mut impl Write) -> Result<()> {
        match *node {
            // Comments are ignored when rendering
            Node::Comment(_, _) => (),
            Node::Text(ref s) | Node::Raw(_, ref s, _) => write!(write, "{}", s)?,
            Node::VariableBlock(_, ref expr) => self.eval_expression(expr)?.render(write)?,
            Node::Set(_, ref set) => self.eval_set(set)?,
            Node::FilterSection(_, FilterSection { ref filter, ref body }, _) => {
                let body = render_to_string(
                    || format!("filter {}", filter.name),
                    |w| self.render_body(body, w),
                )?;
                // the safe filter doesn't actually exist
                if filter.name == "safe" {
                    write!(write, "{}", body)?;
                } else {
                    self.eval_filter(&Cow::Owned(Value::String(body)), filter, &mut false)?
                        .render(write)?;
                }
            }
            // Macros have been imported at the beginning
            Node::ImportMacro(_, _, _) => (),
            Node::If(ref if_node, _) => self.render_if_node(if_node, write)?,
            Node::Forloop(_, ref forloop, _) => self.render_for_loop(forloop, write)?,
            Node::Break(_) => {
                self.call_stack.break_for_loop()?;
            }
            Node::Continue(_) => {
                self.call_stack.continue_for_loop()?;
            }
            Node::Block(_, ref block, _) => self.render_block(block, 0, write)?,
            Node::Super => self.do_super(write)?,
            Node::Include(_, ref tpl_names, ignore_missing) => {
                let mut found = false;
                for tpl_name in tpl_names {
                    let template = self.tera.get_template(tpl_name);
                    if template.is_err() {
                        continue;
                    }
                    let template = template.unwrap();
                    self.macros.add_macros_from_template(self.tera, template)?;
                    self.call_stack.push_include_frame(tpl_name, template);
                    self.render_body(&template.ast, write)?;
                    self.call_stack.pop();
                    found = true;
                    break;
                }
                if !found && !ignore_missing {
                    return Err(Error::template_not_found(
                        ["[", &tpl_names.join(", "), "]"].join(""),
                    ));
                }
            }
            Node::Extends(_, ref name) => {
                return Err(Error::msg(format!(
                    "Inheritance in included templates is currently not supported: extended `{}`",
                    name
                )));
            }
            // Macro definitions are ignored when rendering
            Node::MacroDefinition(_, _, _) => (),
        };

        Ok(())
    }

    /// Helper fn that tries to find the current context: are we in a macro? in a parent template?
    /// in order to give the best possible error when getting an error when rendering a tpl
    fn get_error_location(&self) -> String {
        let mut error_location = format!("Failed to render '{}'", self.template.name);

        // in a macro?
        if self.call_stack.current_frame().kind == FrameType::Macro {
            let frame = self.call_stack.current_frame();
            error_location += &format!(
                ": error while rendering macro `{}::{}`",
                frame.macro_namespace.expect("Macro namespace"),
                frame.name,
            );
        }

        // which template are we in?
        if let Some(&(name, _template, ref level)) = self.blocks.last() {
            let block_def =
                self.template.blocks_definitions.get(&name.to_string()).and_then(|b| b.get(*level));

            if let Some((tpl_name, _)) = block_def {
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

    /// Entry point for the rendering
    pub fn render(&mut self, write: &mut impl Write) -> Result<()> {
        for node in &self.template_root.ast {
            self.render_node(node, write)
                .map_err(|e| Error::chain(self.get_error_location(), e))?;
        }

        Ok(())
    }
}
