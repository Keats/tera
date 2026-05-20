//! AST -> bytecode

use std::collections::HashSet;

use crate::HashMap;
use crate::parsing::ast::{
    ArrayEntry, BinaryOperator, Block, Expression, MapEntry, Node, UnaryOperator,
};
use crate::parsing::instructions::{Chunk, Instruction};
use crate::utils::Span;
use crate::value::Value;

/// We need to handle some pc jumps but we only know to where after we are done processing it
#[derive(Debug)]
enum ProcessingBody {
    /// if/elif
    Branch(usize),
    /// and/or
    ShortCircuit(Vec<usize>),
    Loop(usize),
}

pub(crate) struct Compiler {
    pub(crate) chunk: Chunk,
    processing_bodies: Vec<ProcessingBody>,
    /// The actual blocks definition
    pub(crate) blocks: HashMap<String, Chunk>,
    /// Tracks top-level block definitions with their spans for validation.
    pub(crate) block_name_spans: HashMap<String, Span>,
    /// The current block nesting depth for determining if a block is top-level
    block_depth: usize,
    /// Tracks all various calls with their location for error reporting
    pub(crate) component_calls: HashMap<String, Vec<Span>>,
    pub(crate) filter_calls: HashMap<String, Vec<Span>>,
    pub(crate) test_calls: HashMap<String, Vec<Span>>,
    pub(crate) function_calls: HashMap<String, Vec<Span>>,
    pub(crate) include_calls: HashMap<String, Vec<Span>>,
    pub(crate) top_level_variables: HashSet<String>,
    /// Represents variables set by a loop or by set
    pub(crate) temp_variables: Vec<HashSet<String>>,
    pub(crate) raw_content_num_bytes: usize,
}

impl Compiler {
    pub(crate) fn new(name: &str) -> Self {
        Self {
            chunk: Chunk::new(name),
            processing_bodies: Vec::new(),
            component_calls: HashMap::new(),
            filter_calls: HashMap::new(),
            test_calls: HashMap::new(),
            function_calls: HashMap::new(),
            include_calls: HashMap::new(),
            blocks: HashMap::new(),
            block_name_spans: HashMap::new(),
            top_level_variables: HashSet::default(),
            temp_variables: vec![HashSet::new()],
            block_depth: 0,
            raw_content_num_bytes: 0,
        }
    }

    fn compile_kwargs(&mut self, kwargs: HashMap<String, Expression>) {
        let num_args = kwargs.len();
        // TODO: push a single instr for all keys as a Vec<String> like Python? bench first
        for (key, value) in kwargs {
            self.chunk.add(
                Instruction::LoadConst(Value::from(key)),
                Some(value.span().clone()),
            );
            self.compile_expr(value);
        }
        self.chunk.add(Instruction::BuildMap(num_args), None);
    }

    fn compile_map_entries(&mut self, entries: Vec<MapEntry>, span: Option<Span>) {
        let has_spreads = entries.iter().any(|e| matches!(e, MapEntry::Spread(_)));

        if has_spreads {
            let mut entry_types = Vec::with_capacity(entries.len());
            for entry in entries {
                match entry {
                    MapEntry::KeyValue { key, value } => {
                        self.chunk.add(
                            Instruction::LoadConst(Value::from(key)),
                            Some(value.span().clone()),
                        );
                        self.compile_expr(value);
                        entry_types.push(false);
                    }
                    MapEntry::Spread(expr) => {
                        self.compile_expr(expr);
                        entry_types.push(true);
                    }
                }
            }
            self.chunk
                .add(Instruction::BuildMapWithSpreads(entry_types), span);
        } else {
            let num_items = entries.len();
            for entry in entries {
                if let MapEntry::KeyValue { key, value } = entry {
                    self.chunk.add(
                        Instruction::LoadConst(Value::from(key)),
                        Some(value.span().clone()),
                    );
                    self.compile_expr(value);
                }
            }
            self.chunk.add(Instruction::BuildMap(num_items), span);
        }
    }

    fn compile_expr(&mut self, expr: Expression) {
        match expr {
            Expression::Const(e) => {
                let (val, span) = e.into_parts();
                self.chunk.add(Instruction::LoadConst(val), Some(span));
            }
            Expression::Map(e) => {
                let (map, span) = e.into_parts();
                self.compile_map_entries(map.entries, Some(span));
            }
            Expression::Array(e) => {
                let (array, span) = e.into_parts();
                let has_spreads = array
                    .items
                    .iter()
                    .any(|e| matches!(e, ArrayEntry::Spread(_)));

                if has_spreads {
                    let mut entry_types = Vec::with_capacity(array.items.len());

                    for entry in array.items {
                        match entry {
                            ArrayEntry::Item(expr) => {
                                self.compile_expr(expr);
                                entry_types.push(false);
                            }
                            ArrayEntry::Spread(expr) => {
                                self.compile_expr(expr);
                                entry_types.push(true);
                            }
                        }
                    }

                    self.chunk
                        .add(Instruction::BuildListWithSpreads(entry_types), Some(span));
                } else {
                    let num_elems = array.items.len();
                    for entry in array.items {
                        if let ArrayEntry::Item(expr) = entry {
                            self.compile_expr(expr);
                        }
                    }
                    self.chunk
                        .add(Instruction::BuildList(num_elems), Some(span));
                }
            }
            Expression::Var(e) => {
                let (val, span) = e.into_parts();

                // Ignore our own magic variables
                if !val.name.starts_with("__tera_") {
                    // Also ignore loop scoped vars
                    let from_loop_scope = self
                        .temp_variables
                        .iter()
                        .any(|s| s.contains(val.name.as_str()));
                    if !from_loop_scope {
                        self.top_level_variables.insert(val.name.clone());
                    }
                }

                self.chunk.add(Instruction::LoadName(val.name), Some(span));
            }
            Expression::GetAttr(e) => {
                let (attr, span) = e.into_parts();
                self.compile_expr(attr.expr);
                if attr.optional {
                    self.chunk
                        .add(Instruction::LoadAttrOpt(attr.name), Some(span));
                } else {
                    self.chunk.add(Instruction::LoadAttr(attr.name), Some(span));
                }
            }
            Expression::GetItem(e) => {
                let (item, span) = e.into_parts();
                self.compile_expr(item.expr);
                self.compile_expr(item.sub_expr);
                if item.optional {
                    self.chunk.add(Instruction::BinarySubscriptOpt, Some(span));
                } else {
                    self.chunk.add(Instruction::BinarySubscript, Some(span));
                }
            }
            Expression::Slice(e) => {
                let (slice, span) = e.into_parts();
                self.compile_expr(slice.expr);
                if let Some(start) = slice.start {
                    self.compile_expr(start);
                } else {
                    self.chunk.add(Instruction::LoadConst(Value::none()), None);
                }

                if let Some(end) = slice.end {
                    self.compile_expr(end);
                } else {
                    self.chunk.add(Instruction::LoadConst(Value::none()), None);
                }

                if let Some(step) = slice.step {
                    self.compile_expr(step);
                } else {
                    self.chunk.add(Instruction::LoadConst(1.into()), None);
                }

                if slice.optional {
                    self.chunk.add(Instruction::SliceOpt, Some(span));
                } else {
                    self.chunk.add(Instruction::Slice, Some(span));
                }
            }
            Expression::Filter(e) => {
                let (filter, span) = e.into_parts();
                self.compile_expr(filter.expr);
                self.compile_kwargs(filter.kwargs);
                self.filter_calls
                    .entry(filter.name.clone())
                    .or_default()
                    .push(span.clone());
                self.chunk
                    .add(Instruction::ApplyFilter(filter.name), Some(span));
            }
            Expression::Test(e) => {
                let (test, span) = e.into_parts();
                self.compile_expr(test.expr);
                self.compile_kwargs(test.kwargs);
                self.test_calls
                    .entry(test.name.clone())
                    .or_default()
                    .push(span.clone());
                self.chunk.add(Instruction::RunTest(test.name), Some(span));
            }
            Expression::Ternary(e) => {
                let (ternary, _) = e.into_parts();
                self.compile_expr(ternary.expr);
                let idx = self.chunk.add(Instruction::PopJumpIfFalse(0), None) as usize;
                self.processing_bodies.push(ProcessingBody::Branch(idx));
                self.compile_expr(ternary.true_expr);
                let idx = self.chunk.add(Instruction::Jump(0), None) as usize;
                self.end_branch(self.chunk.len());
                self.processing_bodies.push(ProcessingBody::Branch(idx));
                self.compile_expr(ternary.false_expr);
                self.end_branch(self.chunk.len());
            }
            Expression::ListComprehension(e) => {
                let (list_comp, span) = e.into_parts();

                self.chunk
                    .add(Instruction::BuildList(0), Some(span.clone()));
                self.compile_expr(list_comp.target);
                self.chunk.add(
                    Instruction::StartIterateComprehension(list_comp.key.is_some()),
                    None,
                );
                let mut loop_vars = HashSet::new();
                loop_vars.insert(list_comp.value.clone());
                self.chunk
                    .add(Instruction::StoreLocal(list_comp.value), None);
                if let Some(k) = list_comp.key {
                    loop_vars.insert(k.clone());
                    self.chunk.add(Instruction::StoreLocal(k), None);
                }
                self.temp_variables.push(loop_vars);

                let start_idx = self.chunk.add(Instruction::Iterate(0), None) as usize;
                let cond_skip_idx = if let Some(c) = list_comp.condition {
                    self.compile_expr(c);
                    Some(self.chunk.add(Instruction::PopJumpIfFalse(0), None) as usize)
                } else {
                    None
                };
                self.compile_expr(list_comp.expr);
                self.chunk.add(Instruction::AppendToList, None);
                if let Some(idx) = cond_skip_idx {
                    let jump_back_target = self.chunk.len();
                    if let Some((Instruction::PopJumpIfFalse(t), _)) = self.chunk.get_mut(idx) {
                        *t = jump_back_target;
                    } else {
                        unreachable!();
                    }
                }
                self.chunk.add(Instruction::Jump(start_idx), None);
                let loop_end = self.chunk.len();
                if let Some((Instruction::Iterate(t), _)) = self.chunk.get_mut(start_idx) {
                    *t = loop_end;
                } else {
                    unreachable!();
                }

                self.chunk.add(Instruction::PopLoop, None);
                self.temp_variables.pop();
            }
            Expression::ComponentCall(e) => {
                let (component_call, span) = e.into_parts();

                // Record the component call for validation
                self.component_calls
                    .entry(component_call.name.clone())
                    .or_default()
                    .push(span.clone());

                if !component_call.self_closing {
                    self.chunk.add(Instruction::Capture, None);
                    for node in component_call.body {
                        self.compile_node(node);
                    }
                    self.chunk.add(Instruction::EndCapture, None);
                }

                self.compile_map_entries(component_call.kwargs, None);

                if component_call.self_closing {
                    self.chunk.add(
                        Instruction::RenderInlineComponent(component_call.name),
                        Some(span),
                    );
                } else {
                    self.chunk.add(
                        Instruction::RenderBodyComponent(component_call.name),
                        Some(span),
                    );
                }
            }
            Expression::FunctionCall(e) => {
                let (func, span) = e.into_parts();
                self.compile_kwargs(func.kwargs);
                self.function_calls
                    .entry(func.name.clone())
                    .or_default()
                    .push(span.clone());
                self.chunk
                    .add(Instruction::CallFunction(func.name), Some(span));
            }
            Expression::UnaryOperation(e) => {
                let (op, span) = e.into_parts();
                self.compile_expr(op.expr);
                match op.op {
                    UnaryOperator::Not => self.chunk.add(Instruction::Not, Some(span)),
                    UnaryOperator::Minus => self.chunk.add(Instruction::Negative, Some(span)),
                };
            }
            Expression::BinaryOperation(e) => {
                let (op, span) = e.into_parts();
                let instr = match op.op {
                    BinaryOperator::Mul => Instruction::Mul,
                    BinaryOperator::Div => Instruction::Div,
                    BinaryOperator::Mod => Instruction::Mod,
                    BinaryOperator::Plus => Instruction::Plus,
                    BinaryOperator::Minus => Instruction::Minus,
                    BinaryOperator::FloorDiv => Instruction::FloorDiv,
                    BinaryOperator::Power => Instruction::Power,
                    BinaryOperator::LessThan => Instruction::LessThan,
                    BinaryOperator::GreaterThan => Instruction::GreaterThan,
                    BinaryOperator::LessThanOrEqual => Instruction::LessThanOrEqual,
                    BinaryOperator::GreaterThanOrEqual => Instruction::GreaterThanOrEqual,
                    BinaryOperator::Equal => Instruction::Equal,
                    BinaryOperator::NotEqual => Instruction::NotEqual,
                    BinaryOperator::StrConcat => Instruction::StrConcat,
                    BinaryOperator::In => Instruction::In,
                    BinaryOperator::And | BinaryOperator::Or => {
                        self.processing_bodies
                            .push(ProcessingBody::ShortCircuit(vec![]));
                        self.compile_expr(op.left);
                        if let Some(ProcessingBody::ShortCircuit(instr)) =
                            self.processing_bodies.last_mut()
                        {
                            instr.push(self.chunk.add(
                                if op.op == BinaryOperator::And {
                                    Instruction::JumpIfFalseOrPop(0)
                                } else {
                                    Instruction::JumpIfTrueOrPop(0)
                                },
                                None,
                            ) as usize);
                        } else {
                            unreachable!();
                        }
                        self.compile_expr(op.right);
                        let end = self.chunk.len();
                        if let Some(ProcessingBody::ShortCircuit(instr)) =
                            self.processing_bodies.pop()
                        {
                            for i in instr {
                                match self.chunk.get_mut(i) {
                                    Some((Instruction::JumpIfFalseOrPop(target), _))
                                    | Some((Instruction::JumpIfTrueOrPop(target), _)) => {
                                        *target = end;
                                    }
                                    _ => {}
                                }
                            }
                        } else {
                            unreachable!()
                        }
                        return;
                    }
                    // These are not really binops and we already switched them to separate AST
                    // nodes in the parser so we are not going to have them here
                    BinaryOperator::Is | BinaryOperator::Pipe => unreachable!(),
                };
                // TODO: implement constant folding for arithmetics, string concat
                // Value::constant_fold(self, other) -> Option<Value>?
                // need to pass the op as well...^
                self.compile_expr(op.left);
                self.compile_expr(op.right);
                self.chunk.add(instr, Some(span));
            }
        }
    }

    fn compile_block(&mut self, block: Block) {
        let (block_name, block_span) = block.name.into_parts();
        let is_top_level = self.block_depth == 0;

        let chunk_name = self.chunk.name.clone();
        let parent_chunk = std::mem::replace(&mut self.chunk, Chunk::new(&chunk_name));
        let parent_bodies = std::mem::take(&mut self.processing_bodies);
        self.block_depth += 1;
        for node in block.body {
            self.compile_node(node);
        }
        self.block_depth -= 1;
        let block_chunk = std::mem::replace(&mut self.chunk, parent_chunk);
        self.processing_bodies = parent_bodies;

        if is_top_level {
            self.block_name_spans.insert(block_name.clone(), block_span);
        }
        self.blocks.insert(block_name.clone(), block_chunk);
        self.chunk.add(Instruction::RenderBlock(block_name), None);
    }

    fn end_branch(&mut self, idx: usize) {
        match self.processing_bodies.pop() {
            Some(ProcessingBody::Branch(instr)) => match self.chunk.get_mut(instr) {
                Some((Instruction::Jump(target), _))
                | Some((Instruction::PopJumpIfFalse(target), _)) => {
                    *target = idx;
                }
                _ => {}
            },
            _ => unreachable!(),
        }
    }

    fn get_current_loop(&self) -> Option<&ProcessingBody> {
        self.processing_bodies
            .iter()
            .rev()
            .find(|b| matches!(b, ProcessingBody::Loop(..)))
    }

    pub fn compile_node(&mut self, node: Node) {
        match node {
            Node::Content(text) => {
                self.raw_content_num_bytes += text.len();
                self.chunk.add(Instruction::WriteText(text), None);
            }
            Node::Expression(expr) => {
                self.compile_expr(expr);
                self.chunk.add(Instruction::WriteTop, None);
            }
            Node::Set(s) => {
                self.compile_expr(s.value);
                let scope = if s.global {
                    self.temp_variables.first_mut()
                } else {
                    self.temp_variables.last_mut()
                };
                scope.unwrap().insert(s.name.clone());
                let instr = if s.global {
                    Instruction::SetGlobal(s.name)
                } else {
                    Instruction::Set(s.name)
                };
                self.chunk.add(instr, None);
            }
            Node::BlockSet(b) => {
                self.chunk.add(Instruction::Capture, None);
                for node in b.body {
                    self.compile_node(node);
                }
                self.chunk.add(Instruction::EndCapture, None);
                for expr in b.filters {
                    if let Expression::Filter(f) = expr {
                        let (filter, span) = f.into_parts();
                        self.compile_kwargs(filter.kwargs);
                        self.filter_calls
                            .entry(filter.name.clone())
                            .or_default()
                            .push(span);
                        self.chunk.add(Instruction::ApplyFilter(filter.name), None);
                    }
                }
                let scope = if b.global {
                    self.temp_variables.first_mut()
                } else {
                    self.temp_variables.last_mut()
                };
                scope.unwrap().insert(b.name.clone());
                let instr = if b.global {
                    Instruction::SetGlobal(b.name)
                } else {
                    Instruction::Set(b.name)
                };
                self.chunk.add(instr, None);
            }
            Node::Include(i) => {
                let (name, span) = i.name.into_parts();
                self.include_calls
                    .entry(name.clone())
                    .or_default()
                    .push(span.clone());
                self.chunk.add(Instruction::Include(name), Some(span));
            }
            Node::Block(b) => {
                self.compile_block(b);
            }
            Node::ForLoop(forloop) => {
                self.compile_expr(forloop.target);
                self.chunk
                    .add(Instruction::StartIterate(forloop.key.is_some()), None);
                // The value is sent before the key to be consistent with a value only loop
                let mut loop_vars = HashSet::new();
                loop_vars.insert(forloop.value.clone());
                self.chunk.add(Instruction::StoreLocal(forloop.value), None);
                if let Some(key_var) = forloop.key {
                    loop_vars.insert(key_var.clone());
                    self.chunk.add(Instruction::StoreLocal(key_var), None);
                }
                self.temp_variables.push(loop_vars);

                let start_idx = self.chunk.add(Instruction::Iterate(0), None) as usize;
                self.processing_bodies.push(ProcessingBody::Loop(start_idx));

                for node in forloop.body {
                    self.compile_node(node);
                }

                let has_else = !forloop.else_body.is_empty();

                match self.processing_bodies.pop() {
                    Some(ProcessingBody::Loop(start_idx)) => {
                        self.chunk.add(Instruction::Jump(start_idx), None);
                        let loop_end = self.chunk.len();

                        if has_else {
                            self.chunk.add(Instruction::StoreDidNotIterate, None);
                        }

                        self.chunk.add(Instruction::PopLoop, None);
                        if let Some((Instruction::Iterate(jump_target), _)) =
                            self.chunk.get_mut(start_idx)
                        {
                            *jump_target = loop_end;
                        } else {
                            unreachable!();
                        }
                    }
                    _ => unreachable!(),
                }

                // Pop the loop scope so loop variables don't leak out
                self.temp_variables.pop();

                if has_else {
                    let idx = self.chunk.add(Instruction::PopJumpIfFalse(0), None) as usize;
                    self.processing_bodies.push(ProcessingBody::Branch(idx));
                    for node in forloop.else_body {
                        self.compile_node(node);
                    }
                    self.end_branch(self.chunk.len());
                }
            }
            Node::Break => {
                self.chunk.add(Instruction::Break, None);
            }
            Node::Continue => {
                if let ProcessingBody::Loop(idx) = self.get_current_loop().unwrap() {
                    self.chunk.add(Instruction::Jump(*idx), None);
                }
            }
            Node::If(i) => {
                self.compile_expr(i.expr);

                let idx = self.chunk.add(Instruction::PopJumpIfFalse(0), None) as usize;
                self.processing_bodies.push(ProcessingBody::Branch(idx));

                for node in i.body {
                    self.compile_node(node);
                }

                if !i.false_body.is_empty() {
                    let idx = self.chunk.add(Instruction::Jump(0), None) as usize;
                    self.end_branch(self.chunk.len());
                    self.processing_bodies.push(ProcessingBody::Branch(idx));

                    for node in i.false_body {
                        self.compile_node(node);
                    }
                }

                self.end_branch(self.chunk.len());
            }
            Node::FilterSection(f) => {
                self.chunk.add(Instruction::Capture, None);
                for node in f.body {
                    self.compile_node(node);
                }
                self.chunk.add(Instruction::EndCapture, None);
                self.compile_kwargs(f.kwargs);
                let (filter_name, span) = f.name.into_parts();
                self.filter_calls
                    .entry(filter_name.clone())
                    .or_default()
                    .push(span);
                self.chunk.add(Instruction::ApplyFilter(filter_name), None);
                self.chunk.add(Instruction::WriteTop, None);
            }
        }
    }

    pub fn compile(&mut self, nodes: Vec<Node>) {
        for node in nodes {
            self.compile_node(node);
        }
    }
}
