use std::io::Write;
use std::sync::Arc;

use crate::errors::{Error, ErrorKind, ReportError, TeraResult};
use crate::parsing::{Chunk, Instruction};
use crate::template::Template;
use crate::utils::Span;
use crate::value::{Key, Value, ValueInner};
use crate::vm::for_loop::ForLoop;
use crate::vm::stack::{SpanRange, combine_spans};

use crate::args::Kwargs;
use crate::vm::state::{MAGICAL_DUMP_VAR, State};
use crate::{Context, Tera};

pub(crate) struct VirtualMachine<'tera> {
    tera: &'tera Tera,
    template: &'tera Template,
    /// Only used when rendering a single component, to decide whether to auto-escape it or not
    autoescape_override: Option<bool>,
}

impl<'tera> VirtualMachine<'tera> {
    pub fn new(tera: &'tera Tera, template: &'tera Template) -> Self {
        Self {
            tera,
            template,
            autoescape_override: None,
        }
    }

    pub fn new_with_autoescape(
        tera: &'tera Tera,
        template: &'tera Template,
        autoescape: bool,
    ) -> Self {
        Self {
            tera,
            template,
            autoescape_override: Some(autoescape),
        }
    }

    fn autoescape_enabled(&self) -> bool {
        self.autoescape_override
            .unwrap_or(self.template.autoescape_enabled)
    }

    pub(crate) fn interpret(
        &self,
        state: &mut State<'tera>,
        output: &mut impl Write,
    ) -> TeraResult<()> {
        let mut ip = 0;

        macro_rules! rendering_error {
            ($msg:expr,$span_range:expr) => {{
                let chunk = state.chunk.expect("to have a chunk");
                let span = $span_range
                    .as_ref()
                    .and_then(|r| chunk.expand_span(r))
                    .expect("to have a span for error");
                let (name, source) = if self.template.name != chunk.name {
                    let tpl = &self.tera.templates[&chunk.name];
                    (&tpl.name, &tpl.source)
                } else {
                    (&self.template.name, &self.template.source)
                };
                let err = ReportError::new($msg, name, source, &span);
                return Err(Error::new(ErrorKind::RenderingError(Box::new(err))));
            }};
            // Variant for fused instructions that takes a direct span
            ($msg:expr, span: $span:expr) => {{
                let chunk = state.chunk.expect("to have a chunk");
                let span = $span.expect("to have a span for error");
                let (name, source) = if self.template.name != chunk.name {
                    let tpl = &self.tera.templates[&chunk.name];
                    (&tpl.name, &tpl.source)
                } else {
                    (&self.template.name, &self.template.source)
                };
                let err = ReportError::new($msg, name, source, span);
                return Err(Error::new(ErrorKind::RenderingError(Box::new(err))));
            }};
        }

        macro_rules! op_binop {
            ($op:tt) => {{
                let (b, b_span) = state.stack.pop();
                let (a, a_span) = state.stack.pop();
                state.stack.push(Value::from(a $op b), combine_spans(&a_span, &b_span));
            }};
        }

        macro_rules! math_binop {
            ($fn:ident) => {{
                let (b, b_span) = state.stack.pop();
                let (a, a_span) = state.stack.pop();

                if !a.is_number() {
                    rendering_error!(
                        format!(
                            "Math operations can only be done on numbers, found `{}`",
                            a.name()
                        ),
                        a_span
                    );
                }

                if !b.is_number() {
                    rendering_error!(
                        format!(
                            "Math operations can only be done on numbers, found `{}`",
                            b.name()
                        ),
                        b_span
                    );
                }

                let c_span = combine_spans(&a_span, &b_span);
                match crate::value::number::$fn(&a, &b) {
                    Ok(c) => state.stack.push(c, c_span),
                    Err(e) => {
                        let err_msg = e.to_string();
                        // yucky
                        if err_msg.contains("divide by 0") {
                            rendering_error!(err_msg, b_span);
                        } else {
                            rendering_error!(err_msg, c_span);
                        }
                    }
                }
            }};
        }

        macro_rules! component {
            ($name:expr, $span_idx:expr, $has_body:expr) => {{
                let (kwargs, _) = state.stack.pop();
                let kwargs = kwargs.into_map().expect("to have kwargs");
                let (component_def, component_chunk) = &self.tera.components[$name];
                let current_span: SpanRange = Some($span_idx..=$span_idx);

                let body = if $has_body {
                    Some(state.stack.pop().0.mark_safe())
                } else {
                    None
                };

                let context = match component_def.build_context(
                    kwargs.keys().filter_map(|k| k.as_str()),
                    |key| kwargs.get(&Key::Str(key)).cloned(),
                    body,
                ) {
                    Ok(ctx) => ctx,
                    Err(msg) => rendering_error!(msg, current_span),
                };

                let val = match self.render_component(&component_chunk, context) {
                    Ok(v) => v,
                    Err(mut e) => {
                        if let ErrorKind::RenderingError(ref mut report) = e.kind {
                            let chunk = state.chunk.expect("to have a chunk");
                            if let Some(span) =
                                current_span.as_ref().and_then(|r| chunk.expand_span(r))
                            {
                                report.add_note(
                                    "called from",
                                    &self.template.name,
                                    &self.template.source,
                                    &span,
                                );
                            }
                        }
                        return Err(e);
                    }
                };
                state.stack.push(Value::safe_string(&val), current_span);
            }};
        }

        while let Some((instr, _)) = state.chunk.expect("To have a chunk").get(ip) {
            // Current instruction index as span reference
            let current_ip = ip as u32;

            match instr {
                Instruction::LoadConst(v) => {
                    state.stack.push(v.clone(), Some(current_ip..=current_ip));
                }
                Instruction::LoadName(n) => state.load_name(n, current_ip),
                Instruction::LoadAttr(attr) | Instruction::LoadAttrOpt(attr) => {
                    let is_optional = matches!(instr, Instruction::LoadAttrOpt(_));
                    let (a, a_span) = state.stack.pop();
                    if is_optional && (a.is_undefined() || a.is_none()) {
                        state
                            .stack
                            .push(Value::undefined(), Some(current_ip..=current_ip));
                    } else {
                        if a.is_undefined() {
                            rendering_error!(format!("Field `{}` is not defined", attr), a_span);
                        }
                        let next = a.get_attr(attr).cloned().unwrap_or_else(Value::undefined);
                        state.stack.push(next, Some(current_ip..=current_ip));
                    }
                }
                Instruction::BinarySubscript | Instruction::BinarySubscriptOpt => {
                    let is_optional = matches!(instr, Instruction::BinarySubscriptOpt);
                    let (subscript, subscript_span) = state.stack.pop();
                    let (val, val_span) = state.stack.pop();
                    if is_optional && (val.is_undefined() || val.is_none()) {
                        state
                            .stack
                            .push(Value::undefined(), Some(current_ip..=current_ip));
                    } else {
                        if val.is_undefined() {
                            rendering_error!(
                                "Cannot index into an undefined value".to_owned(),
                                val_span
                            );
                        }
                        if subscript.is_undefined() {
                            rendering_error!(
                                "Index expression is undefined".to_owned(),
                                subscript_span
                            );
                        }

                        let c_span = combine_spans(&val_span, &subscript_span);
                        match val.get_item(subscript) {
                            Ok(v) => {
                                state.stack.push(v, c_span);
                            }
                            Err(e) => {
                                rendering_error!(e.to_string(), subscript_span);
                            }
                        }
                    }
                }
                Instruction::Slice | Instruction::SliceOpt => {
                    let is_optional = matches!(instr, Instruction::SliceOpt);
                    let (step, _) = state.stack.pop();
                    let (end, _) = state.stack.pop();
                    let (start, _) = state.stack.pop();
                    let (val, val_span) = state.stack.pop();
                    if is_optional && val.is_undefined() {
                        state
                            .stack
                            .push(Value::undefined(), Some(current_ip..=current_ip));
                    } else {
                        if val.is_undefined() {
                            rendering_error!(
                                "Cannot slice an undefined value".to_owned(),
                                val_span
                            );
                        }

                        // This returns an error if the value is not an array/string so we don't need to
                        // expand the span.
                        match val.slice(start.as_i128(), end.as_i128(), step.as_i128()) {
                            Ok(v) => {
                                state.stack.push(v, val_span);
                            }
                            Err(e) => {
                                rendering_error!(e.to_string(), val_span);
                            }
                        }
                    }
                }
                Instruction::WriteText(t) => {
                    if let Some(captured) = state.capture_buffers.last_mut() {
                        captured.write_all(t.as_bytes())?;
                    } else {
                        output.write_all(t.as_bytes())?;
                    }
                }
                Instruction::WriteTop => {
                    let (top, top_span) = state.stack.pop();
                    if top.is_undefined() {
                        rendering_error!(
                            format!("Tried to render a variable that is not defined"),
                            top_span
                        );
                    }

                    if !self.autoescape_enabled() || top.is_safe() {
                        if let Some(captured) = state.capture_buffers.last_mut() {
                            top.format(captured)?;
                        } else {
                            top.format(output)?;
                        }
                    } else {
                        // Avoiding String as much as possible
                        state.escape_buffer.clear();
                        top.format(&mut state.escape_buffer)?;
                        if let Some(captured) = state.capture_buffers.last_mut() {
                            (self.tera.escape_fn)(&state.escape_buffer, captured)?;
                        } else {
                            (self.tera.escape_fn)(&state.escape_buffer, output)?;
                        }
                    }
                }
                Instruction::Set(name) => {
                    let (val, _) = state.stack.pop();
                    state.store_local(name, val);
                }
                Instruction::SetGlobal(name) => {
                    let (val, _) = state.stack.pop();
                    state.store_global(name, val);
                }
                Instruction::Include(name) => {
                    if let Err(mut e) = self.render_include(name, state, output) {
                        if let ErrorKind::RenderingError(ref mut report) = e.kind {
                            let chunk = state.chunk.expect("to have a chunk");
                            if let Some(span) = chunk.get_span(current_ip) {
                                report.add_note(
                                    "called from",
                                    &self.template.name,
                                    &self.template.source,
                                    span,
                                );
                            }
                        }
                        return Err(e);
                    }
                }
                Instruction::BuildMap(num_elem) => {
                    let mut elems = Vec::with_capacity(*num_elem);
                    for _ in 0..*num_elem {
                        let (val, _) = state.stack.pop();
                        let (key, _) = state.stack.pop();
                        elems.push((key.as_key()?, val));
                    }
                    elems.reverse();
                    let map: crate::value::Map = elems.into_iter().collect();
                    state.stack.push(Value::from(map), None)
                }
                Instruction::BuildMapWithSpreads(entry_types) => {
                    let mut result_map = crate::value::Map::new();

                    // We process the values from right to left because right will always win
                    // against the same key/val on the left so we don't need to insert multiple times
                    for is_spread in entry_types.iter().rev() {
                        if *is_spread {
                            let (val, span) = state.stack.pop();
                            if !val.is_map() {
                                rendering_error!(
                                    format!(
                                        "Spread operator requires a map, found `{}`",
                                        val.name()
                                    ),
                                    span
                                );
                            }
                            for (k, v) in val.into_map().unwrap() {
                                result_map.entry(k).or_insert(v);
                            }
                        } else {
                            let (val, _) = state.stack.pop();
                            let (key, _) = state.stack.pop();
                            result_map.entry(key.as_key()?).or_insert(val);
                        }
                    }

                    state.stack.push(Value::from(result_map), None);
                }
                Instruction::BuildList(num_elem) => {
                    let mut elems = Vec::with_capacity(*num_elem);
                    for _ in 0..*num_elem {
                        elems.push(state.stack.pop().0);
                    }
                    elems.reverse();
                    state.stack.push(Value::from(elems), None);
                }
                Instruction::BuildListWithSpreads(entry_types) => {
                    let mut result = Vec::with_capacity(entry_types.len());
                    for is_spread in entry_types.iter().rev() {
                        let (val, span) = state.stack.pop();
                        if *is_spread {
                            if !val.is_array() {
                                rendering_error!(
                                    format!(
                                        "Spread operator requires an array, found `{}`",
                                        val.name()
                                    ),
                                    span
                                );
                            }
                            for item in val.into_vec().unwrap().into_iter().rev() {
                                result.push(item);
                            }
                        } else {
                            result.push(val);
                        }
                    }
                    result.reverse();

                    state.stack.push(Value::from(result), None);
                }
                Instruction::CallFunction(name) => {
                    let (kwargs, _) = state.stack.pop();
                    if name == "super" {
                        let current_block_name =
                            state.current_block_name.expect("no current block");
                        let (blocks, level) = state
                            .blocks
                            .remove(current_block_name)
                            .expect("no lineage found");
                        if blocks.len() == 1 {
                            rendering_error!(
                                format!("Tried to use super() in the top level block"),
                                Some(current_ip..=current_ip)
                            );
                        }

                        let block_chunk = blocks[level + 1];
                        let old_chunk = state.chunk.replace(block_chunk);
                        state.blocks.insert(current_block_name, (blocks, level + 1));
                        let res = self.interpret(state, output);
                        state.chunk = old_chunk;
                        res?;
                        state.stack.push(Value::none(), None);
                    } else {
                        let f = &self.tera.functions[name.as_str()];
                        let val = match f
                            .call(Kwargs::new(Arc::new(kwargs.into_map().unwrap())), state)
                        {
                            Ok(v) => v,
                            Err(err) => {
                                rendering_error!(format!("{err}"), Some(current_ip..=current_ip))
                            }
                        };
                        let val = if f.is_safe() { val.mark_safe() } else { val };
                        state.stack.push(val, Some(current_ip..=current_ip));
                    }
                }
                Instruction::ApplyFilter(name) => {
                    let f = &self.tera.filters[name.as_str()];
                    let (kwargs, _) = state.stack.pop();
                    let (value, value_span) = state.stack.pop();
                    let val = match f.call(
                        &value,
                        Kwargs::new(Arc::new(kwargs.into_map().unwrap())),
                        state,
                    ) {
                        Ok(v) => v,
                        Err(err) => match err.kind {
                            ErrorKind::InvalidArgument { .. } => {
                                rendering_error!(format!("{err}"), value_span)
                            }
                            _ => rendering_error!(format!("{err}"), Some(current_ip..=current_ip)),
                        },
                    };
                    let val = if f.is_safe() { val.mark_safe() } else { val };
                    state.stack.push(val, Some(current_ip..=current_ip));
                }
                Instruction::RunTest(name) => {
                    let f = &self.tera.tests[name.as_str()];
                    let (kwargs, _) = state.stack.pop();
                    let (value, value_span) = state.stack.pop();
                    let val = match f.call(
                        &value,
                        Kwargs::new(Arc::new(kwargs.into_map().unwrap())),
                        state,
                    ) {
                        Ok(v) => v,
                        Err(err) => match err.kind {
                            ErrorKind::InvalidArgument { .. } => {
                                rendering_error!(format!("{err}"), value_span)
                            }
                            _ => rendering_error!(format!("{err}"), Some(current_ip..=current_ip)),
                        },
                    };

                    state.stack.push(val.into(), Some(current_ip..=current_ip));
                }
                Instruction::RenderBodyComponent(name) => {
                    component!(name, current_ip, true);
                }
                Instruction::RenderInlineComponent(name) => {
                    component!(name, current_ip, false);
                }
                Instruction::RenderBlock(block_name) => {
                    let block_lineage: Vec<_> = self
                        .template
                        .block_lineage
                        .get(block_name)
                        .map(|bl| bl.iter().collect())
                        .unwrap_or_default();
                    if block_lineage.is_empty() {
                        return Err(Error::message(format!(
                            "Block '{}' has no block lineage in template '{}'. \
                            This usually means the template was not properly finalized.",
                            block_name, self.template.name
                        )));
                    }
                    let block_chunk = block_lineage[0];
                    let old_chunk = state.chunk.replace(block_chunk);
                    state.blocks.insert(block_name, (block_lineage, 0));
                    let old_block_name = state.current_block_name.replace(block_name);
                    let res = self.interpret(state, output);
                    state.chunk = old_chunk;
                    state.current_block_name = old_block_name;
                    res?;
                }
                Instruction::Jump(target_ip) => {
                    ip = *target_ip;
                    continue;
                }
                Instruction::PopJumpIfFalse(target_ip) => {
                    let (val, _) = state.stack.pop();
                    if !val.is_truthy() {
                        ip = *target_ip;
                        continue;
                    }
                }
                Instruction::JumpIfFalseOrPop(target_ip) => {
                    let (peeked, _) = state.stack.peek();
                    if !peeked.is_truthy() {
                        ip = *target_ip;
                        continue;
                    } else {
                        state.stack.pop();
                    }
                }
                Instruction::JumpIfTrueOrPop(target_ip) => {
                    let (peeked, _) = state.stack.peek();
                    if peeked.is_truthy() {
                        ip = *target_ip;
                        continue;
                    } else {
                        state.stack.pop();
                    }
                }
                Instruction::Capture => {
                    state.capture_buffers.push(Vec::with_capacity(128));
                }
                Instruction::EndCapture => {
                    let captured = state.capture_buffers.pop().unwrap();
                    let val = Value::from(String::from_utf8(captured)?);
                    state.stack.push(val, None);
                }
                Instruction::StartIterate(is_key_value) => {
                    let (container, container_span) = state.stack.pop();
                    if !container.can_be_iterated_on() {
                        rendering_error!(
                            format!("Iteration not possible on type `{}`", container.name()),
                            container_span
                        );
                    }

                    if *is_key_value && !container.is_map() {
                        rendering_error!(
                            format!(
                                "Key/value iteration is not possible on type `{}`, only on maps.",
                                container.name()
                            ),
                            container_span
                        );
                    }

                    state.for_loops.push(ForLoop::new(container));
                }
                Instruction::StoreLocal(name) => {
                    if let Some(for_loop) = state.for_loops.last_mut() {
                        for_loop.store_local(name.as_str());
                    }
                }
                Instruction::Iterate(end_ip) => {
                    if let Some(for_loop) = state.for_loops.last_mut() {
                        if for_loop.is_over() {
                            ip = *end_ip;
                            continue;
                        }
                        for_loop.advance();
                        for_loop.end_ip = *end_ip;
                    }
                }
                Instruction::StoreDidNotIterate => {
                    if let Some(for_loop) = state.for_loops.last() {
                        state.stack.push(Value::from(!for_loop.iterated()), None);
                    }
                }
                Instruction::Break => {
                    if let Some(for_loop) = state.for_loops.last_mut() {
                        ip = for_loop.end_ip;
                        continue;
                    }
                }
                Instruction::PopLoop => {
                    state.for_loops.pop();
                }
                Instruction::Mul => math_binop!(mul),
                Instruction::Div => math_binop!(div),
                Instruction::FloorDiv => math_binop!(floor_div),
                Instruction::Mod => math_binop!(rem),
                Instruction::Plus => {
                    let (b, b_span) = state.stack.pop();
                    let (a, a_span) = state.stack.pop();
                    let c_span = combine_spans(&a_span, &b_span);

                    if a.is_number() && b.is_number() {
                        match crate::value::number::add(&a, &b) {
                            Ok(c) => state.stack.push(c, c_span),
                            Err(e) => rendering_error!(e.to_string(), c_span),
                        }
                    } else {
                        rendering_error!(
                            format!(
                                "`+` requires both operands to be numbers, found `{}` and `{}`",
                                a.name(),
                                b.name()
                            ),
                            c_span
                        );
                    }
                }
                Instruction::Minus => math_binop!(sub),
                Instruction::Power => math_binop!(pow),
                Instruction::LessThan => op_binop!(<),
                Instruction::GreaterThan => op_binop!(>),
                Instruction::LessThanOrEqual => op_binop!(<=),
                Instruction::GreaterThanOrEqual => op_binop!(>=),
                Instruction::Equal => op_binop!(==),
                Instruction::NotEqual => op_binop!(!=),
                Instruction::StrConcat => {
                    let (b, b_span) = state.stack.pop();
                    let (a, a_span) = state.stack.pop();
                    let c_span = combine_spans(&a_span, &b_span);

                    let result = match (&a.inner, &b.inner) {
                        (ValueInner::String(a_str), ValueInner::String(b_str)) => {
                            let mut s = String::with_capacity(a_str.len() + b_str.len());
                            s.push_str(a_str.as_str());
                            s.push_str(b_str.as_str());
                            Value::from(s)
                        }
                        _ => Value::from(format!("{a}{b}")),
                    };
                    state.stack.push(result, c_span);
                }
                Instruction::In => {
                    let (container, container_span) = state.stack.pop();
                    let (needle, _) = state.stack.pop();
                    match container.contains(&needle) {
                        Ok(b) => {
                            state.stack.push(Value::from(b), None);
                        }
                        Err(e) => {
                            rendering_error!(e.to_string(), container_span);
                        }
                    };
                }
                Instruction::Not => {
                    let (a, a_span) = state.stack.pop();
                    state.stack.push(Value::from(!a.is_truthy()), a_span);
                }
                Instruction::Negative => {
                    let (a, a_span) = state.stack.pop();
                    match crate::value::number::negate(&a) {
                        Ok(b) => {
                            state.stack.push(b, a_span);
                        }
                        Err(e) => {
                            rendering_error!(e.to_string(), a_span);
                        }
                    }
                }
                // Combined instructions
                Instruction::LoadPath(path) => {
                    let chunk = state.chunk.expect("to have a chunk");
                    let mut val = if path.len() == 1 && path[0] == MAGICAL_DUMP_VAR {
                        state.dump_context()
                    } else {
                        state.get_value(&path[0])
                    };
                    let num_attrs = path.len() - 1;
                    if num_attrs > 0 {
                        if val.is_undefined() {
                            let span = chunk
                                .get_span_at(current_ip, 0)
                                .expect("to have a span for error");
                            return Err(self.undefined_var_error(state, chunk, &path[0], span));
                        }
                        let mut cur: &Value = &val;
                        let mut undefined_tail = false;
                        for (k, attr) in path[1..].iter().enumerate() {
                            match cur.get_attr(attr) {
                                Some(next) => cur = next,
                                None => {
                                    if k + 1 < num_attrs {
                                        let span = chunk
                                            .get_span_at(current_ip, k + 1)
                                            .expect("to have a span for error");
                                        return Err(
                                            self.undefined_field_error(cur, attr, span, chunk)
                                        );
                                    }
                                    undefined_tail = true;
                                    break;
                                }
                            }
                        }
                        val = if undefined_tail {
                            Value::undefined()
                        } else {
                            cur.clone()
                        };
                    }
                    state.stack.push(val, Some(current_ip..=current_ip));
                }
                Instruction::WritePath(path) => {
                    let chunk = state.chunk.expect("to have a chunk");
                    let root = if path.len() == 1 && path[0] == MAGICAL_DUMP_VAR {
                        state.dump_context()
                    } else {
                        state.get_value(&path[0])
                    };
                    if root.is_undefined() {
                        let span = chunk
                            .get_span_at(current_ip, 0)
                            .expect("to have a span for error");
                        return Err(self.undefined_var_error(state, chunk, &path[0], span));
                    }
                    let num_attrs = path.len() - 1;
                    let val: &Value = if num_attrs > 0 {
                        let mut cur: &Value = &root;
                        for (k, attr) in path[1..].iter().enumerate() {
                            match cur.get_attr(attr) {
                                Some(next) => cur = next,
                                None => {
                                    let span = chunk
                                        .get_span_at(current_ip, k + 1)
                                        .expect("to have a span for error");
                                    return Err(self.undefined_field_error(cur, attr, span, chunk));
                                }
                            }
                        }
                        cur
                    } else {
                        &root
                    };

                    if !self.autoescape_enabled() || val.is_safe() {
                        if let Some(captured) = state.capture_buffers.last_mut() {
                            val.format(captured)?;
                        } else {
                            val.format(output)?;
                        }
                    } else {
                        state.escape_buffer.clear();
                        val.format(&mut state.escape_buffer)?;
                        if let Some(captured) = state.capture_buffers.last_mut() {
                            (self.tera.escape_fn)(&state.escape_buffer, captured)?;
                        } else {
                            (self.tera.escape_fn)(&state.escape_buffer, output)?;
                        }
                    }
                }
            }

            ip += 1;
        }

        Ok(())
    }

    fn undefined_var_error(
        &self,
        state: &State<'tera>,
        chunk: &Chunk,
        name: &str,
        span: &Span,
    ) -> Error {
        let available_vars = state.available_variables();
        let available_msg = if available_vars.is_empty() {
            String::new()
        } else {
            format!(" Available variables: {}", available_vars.join(", "))
        };
        self.rendering_error(
            format!("Variable `{name}` is not defined.{available_msg}"),
            chunk,
            span,
        )
    }

    fn undefined_field_error(
        &self,
        parent: &Value,
        attr: &str,
        span: &Span,
        chunk: &Chunk,
    ) -> Error {
        let available_fields = parent.available_fields();
        let available_msg = if available_fields.is_empty() {
            String::new()
        } else {
            format!(" Available fields: {}", available_fields.join(", "))
        };
        self.rendering_error(
            format!("Field `{attr}` is not defined.{available_msg}"),
            chunk,
            span,
        )
    }

    fn rendering_error(&self, msg: String, chunk: &Chunk, span: &Span) -> Error {
        let (name, source) = if self.template.name != chunk.name {
            let tpl = &self.tera.templates[&chunk.name];
            (&tpl.name, &tpl.source)
        } else {
            (&self.template.name, &self.template.source)
        };
        let err = ReportError::new(msg, name, source, span);
        Error::new(ErrorKind::RenderingError(Box::new(err)))
    }

    fn render_component(&self, chunk: &Chunk, context: Context) -> TeraResult<String> {
        let vm = Self {
            tera: self.tera,
            template: self.template,
            autoescape_override: self.autoescape_override,
        };

        let mut state = State::new_with_chunk(&context, chunk);
        state.filters = Some(&self.tera.filters);
        let mut output = Vec::with_capacity(1024);
        vm.interpret(&mut state, &mut output)?;

        Ok(String::from_utf8(output)?)
    }

    fn render_include(
        &self,
        name: &str,
        state: &State<'tera>,
        output: &mut impl Write,
    ) -> TeraResult<()> {
        let tpl = self.tera.must_get_template(name)?;
        let vm = Self {
            tera: self.tera,
            template: tpl,
            autoescape_override: self.autoescape_override,
        };

        // We create a dummy state for variables to be written to, but we don't keep it around
        let mut include_state = State::new_with_chunk(state.context, &tpl.chunk);
        include_state.include_parent = Some(state);
        include_state.filters = Some(&self.tera.filters);
        vm.interpret(&mut include_state, output)?;
        Ok(())
    }

    pub(crate) fn render(
        &mut self,
        context: &Context,
        global_context: &Context,
    ) -> TeraResult<String> {
        let mut output = Vec::with_capacity(self.template.size_hint());
        self.render_to(context, global_context, &mut output)?;
        Ok(String::from_utf8(output)?)
    }

    pub(crate) fn render_to(
        &mut self,
        context: &Context,
        global_context: &Context,
        mut output: impl Write,
    ) -> TeraResult<()> {
        // TODO(perf): can we optimize this at the bytecode level to avoid hashmap lookups?
        let chunk = if let Some(base_tpl_name) = self.template.parents.first() {
            let tpl = self.tera.must_get_template(base_tpl_name)?;
            &tpl.chunk
        } else {
            &self.template.chunk
        };
        let mut state = State::new_with_chunk(context, chunk);
        state.global_context = Some(global_context);
        state.filters = Some(&self.tera.filters);
        self.interpret(&mut state, &mut output)
    }
}
