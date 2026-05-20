use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::str::FromStr;

use crate::HashMap;
use crate::errors::Error;
use crate::utils::{Span, Spanned};
use crate::value::{Key, Value, ValueInner, format_map};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOperator {
    Not,
    Minus,
}

impl fmt::Display for UnaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use UnaryOperator::*;

        let val = match self {
            Minus => "-",
            Not => "not",
        };
        write!(f, "{val}")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOperator {
    // math
    Mul,
    Div,
    Mod,
    Plus,
    Minus,
    FloorDiv,
    Power,

    // comparison
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
    Equal,
    NotEqual,

    // rest
    And,
    Or,
    StrConcat,
    In,

    // Not binary operators, only there simplicity for precedence in the parser.
    Is,
    Pipe,
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use BinaryOperator::*;

        let val = match self {
            Mul => "*",
            Power => "**",
            Div => "/",
            FloorDiv => "//",
            Mod => "%",
            Plus => "+",
            Minus => "-",
            LessThan => "<",
            GreaterThan => ">",
            LessThanOrEqual => "<=",
            GreaterThanOrEqual => ">=",
            Equal => "==",
            NotEqual => "!=",
            And => "and",
            Or => "or",
            StrConcat => "~",
            In => "in",
            Is => "is",
            Pipe => "|",
        };
        write!(f, "{val}")
    }
}

/// An expression is the node found in variable block, kwargs and conditions.
#[derive(Clone, PartialEq)]
#[allow(missing_docs)]
pub enum Expression {
    /// A constant: string, number, boolean, array or null
    Const(Spanned<Value>),
    /// An array that contains things that we need to look up in the context
    Array(Spanned<Array>),
    /// A hashmap defined in the template where we need to look up in the context
    Map(Spanned<Map>),
    /// A variable to look up in the context.
    Var(Spanned<Var>),
    /// The `.` getter, as in item.field
    GetAttr(Spanned<GetAttr>),
    /// The in brackets getter as in `item[hello * 10]`
    GetItem(Spanned<GetItem>),
    /// A python like slice indexing pattern, like `[1:5:2]`
    Slice(Spanned<Slice>),
    /// my_value | safe(potential="argument") filter
    Filter(Spanned<Filter>),
    /// my_value is defined
    Test(Spanned<Test>),
    /// 'a' if truthy else 'b'
    Ternary(Spanned<Ternary>),
    /// `[id | str for id in ids]`
    ListComprehension(Spanned<ListComprehension>),
    ComponentCall(Spanned<ComponentCall>),
    FunctionCall(Spanned<FunctionCall>),
    UnaryOperation(Spanned<UnaryOperation>),
    BinaryOperation(Spanned<BinaryOperation>),
}

impl Expression {
    pub fn is_literal(&self) -> bool {
        matches!(self, Expression::Const(..))
    }

    pub(crate) fn as_value(&self) -> Option<Value> {
        match self {
            Expression::Const(c) => Some(c.node().clone()),
            _ => None,
        }
    }

    pub fn span(&self) -> &Span {
        match self {
            Expression::Const(s) => s.span(),
            Expression::Map(s) => s.span(),
            Expression::Array(s) => s.span(),
            Expression::Test(s) => s.span(),
            Expression::ComponentCall(s) => s.span(),
            Expression::FunctionCall(s) => s.span(),
            Expression::UnaryOperation(s) => s.span(),
            Expression::BinaryOperation(s) => s.span(),
            Expression::Var(s) => s.span(),
            Expression::GetAttr(s) => s.span(),
            Expression::GetItem(s) => s.span(),
            Expression::Slice(s) => s.span(),
            Expression::Filter(s) => s.span(),
            Expression::Ternary(s) => s.span(),
            Expression::ListComprehension(s) => s.span(),
        }
    }

    pub fn expand_span(&mut self, span: &Span) {
        match self {
            Expression::Const(s) => s.span_mut().expand(span),
            Expression::Map(s) => s.span_mut().expand(span),
            Expression::Array(s) => s.span_mut().expand(span),
            Expression::Test(s) => s.span_mut().expand(span),
            Expression::ComponentCall(s) => s.span_mut().expand(span),
            Expression::FunctionCall(s) => s.span_mut().expand(span),
            Expression::UnaryOperation(s) => s.span_mut().expand(span),
            Expression::BinaryOperation(s) => s.span_mut().expand(span),
            Expression::Var(s) => s.span_mut().expand(span),
            Expression::GetAttr(s) => s.span_mut().expand(span),
            Expression::GetItem(s) => s.span_mut().expand(span),
            Expression::Slice(s) => s.span_mut().expand(span),
            Expression::Filter(s) => s.span_mut().expand(span),
            Expression::Ternary(s) => s.span_mut().expand(span),
            Expression::ListComprehension(s) => s.span_mut().expand(span),
        }
    }
}

impl fmt::Debug for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Const(i) => match &i.node().inner {
                ValueInner::Bool(j) => fmt::Debug::fmt(&Spanned::new(*j, i.span().clone()), f),
                ValueInner::I64(j) => fmt::Debug::fmt(&Spanned::new(*j, i.span().clone()), f),
                ValueInner::F64(j) => fmt::Debug::fmt(&Spanned::new(*j, i.span().clone()), f),
                ValueInner::String(j) => fmt::Debug::fmt(&Spanned::new(j, i.span().clone()), f),
                ValueInner::Array(j) => fmt::Debug::fmt(&Spanned::new(j, i.span().clone()), f),
                ValueInner::Map(j) => fmt::Debug::fmt(&Spanned::new(j, i.span().clone()), f),
                ValueInner::None => fmt::Debug::fmt(&Spanned::new((), i.span().clone()), f),
                _ => unreachable!("{self} is not implemented"),
            },
            Expression::Map(i) => fmt::Debug::fmt(i, f),
            Expression::Array(i) => fmt::Debug::fmt(i, f),
            Expression::Test(i) => fmt::Debug::fmt(i, f),
            Expression::ComponentCall(i) => fmt::Debug::fmt(i, f),
            Expression::Filter(i) => fmt::Debug::fmt(i, f),
            Expression::FunctionCall(i) => fmt::Debug::fmt(i, f),
            Expression::UnaryOperation(i) => fmt::Debug::fmt(i, f),
            Expression::BinaryOperation(i) => fmt::Debug::fmt(i, f),
            Expression::Var(i) => fmt::Debug::fmt(i, f),
            Expression::GetAttr(i) => fmt::Debug::fmt(i, f),
            Expression::GetItem(i) => fmt::Debug::fmt(i, f),
            Expression::Slice(i) => fmt::Debug::fmt(i, f),
            Expression::Ternary(i) => fmt::Debug::fmt(i, f),
            Expression::ListComprehension(i) => fmt::Debug::fmt(i, f),
        }
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Const(i) => match &i.node().inner {
                ValueInner::String(s) => write!(f, "'{}'", *s),
                ValueInner::I64(s) => write!(f, "{}", *s),
                ValueInner::F64(s) => write!(f, "{}", *s),
                ValueInner::U64(s) => write!(f, "{}", *s),
                ValueInner::U128(s) => write!(f, "{}", *s),
                ValueInner::I128(s) => write!(f, "{}", *s),
                ValueInner::Bool(s) => write!(f, "{}", *s),
                ValueInner::Array(s) => {
                    write!(f, "[")?;
                    for (i, elem) in s.iter().enumerate() {
                        if i > 0 && i != s.len() {
                            write!(f, ", ")?;
                        }
                        match &elem.inner {
                            ValueInner::String(t) => write!(f, r#""{t}""#),
                            _ => write!(f, "{elem}"),
                        }?;
                    }
                    write!(f, "]")
                }
                ValueInner::None => write!(f, "null"),
                ValueInner::Undefined => write!(f, "undefined"),
                ValueInner::Bytes(_) => write!(f, "<bytes>"),
                ValueInner::Map(s) => {
                    let mut buf: Vec<u8> = Vec::new();
                    format_map(s, &mut buf).expect("failed to write map to vec");
                    write!(
                        f,
                        "{}",
                        std::str::from_utf8(&buf).expect("valid utf-8 in display")
                    )
                }
            },
            Expression::Map(i) => write!(f, "{}", **i),
            Expression::Array(i) => write!(f, "{}", **i),
            Expression::Test(i) => write!(f, "{}", **i),
            Expression::ComponentCall(i) => write!(f, "{}", **i),
            Expression::Filter(i) => write!(f, "{}", **i),
            Expression::FunctionCall(i) => write!(f, "{}", **i),
            Expression::UnaryOperation(i) => write!(f, "{}", **i),
            Expression::BinaryOperation(i) => write!(f, "{}", **i),
            Expression::Var(i) => write!(f, "{}", **i),
            Expression::GetAttr(i) => write!(f, "{}", **i),
            Expression::GetItem(i) => write!(f, "{}", **i),
            Expression::Slice(i) => write!(f, "{}", **i),
            Expression::Ternary(i) => write!(f, "{}", **i),
            Expression::ListComprehension(i) => write!(f, "{}", **i),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Filter {
    pub expr: Expression,
    pub name: String,
    pub kwargs: HashMap<String, Expression>,
}

impl fmt::Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(| {}", self.expr)?;
        write!(f, " {}", self.name)?;
        write!(f, "{{",)?;
        let mut keys = self.kwargs.keys().collect::<Vec<_>>();
        keys.sort();
        for (i, k) in keys.iter().enumerate() {
            if i == self.kwargs.len() - 1 {
                write!(f, "{}={}", k, self.kwargs[*k])?
            } else {
                write!(f, "{}={}, ", k, self.kwargs[*k])?
            }
        }
        write!(f, "}})",)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnaryOperation {
    pub op: UnaryOperator,
    pub expr: Expression,
}

impl fmt::Display for UnaryOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {})", self.op, self.expr)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BinaryOperation {
    pub op: BinaryOperator,
    pub left: Expression,
    pub right: Expression,
}

impl fmt::Display for BinaryOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {} {})", self.op, self.left, self.right)
    }
}

/// An entry in a map literal - either a key-value pair or a spread expression
#[derive(Clone, Debug, PartialEq)]
pub enum MapEntry {
    /// A regular key-value pair: `key: value`
    KeyValue {
        key: Key<'static>,
        value: Expression,
    },
    /// A spread expression: `...expr`
    Spread(Expression),
}

impl fmt::Display for MapEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MapEntry::KeyValue { key, value } => write!(f, "{key}: {value}"),
            MapEntry::Spread(expr) => write!(f, "...{expr}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Map {
    pub entries: Vec<MapEntry>,
}

impl fmt::Display for Map {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        for (i, entry) in self.entries.iter().enumerate() {
            if i == self.entries.len() - 1 {
                write!(f, "{entry}")?
            } else {
                write!(f, "{entry}, ")?
            }
        }
        write!(f, "}}")
    }
}

/// An entry in an array literal - either a single item or a spread expression
#[derive(Clone, Debug, PartialEq)]
pub enum ArrayEntry {
    /// A single item: `expr`
    Item(Expression),
    /// A spread expression: `...expr`
    Spread(Expression),
}

impl fmt::Display for ArrayEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArrayEntry::Item(expr) => write!(f, "{expr}"),
            ArrayEntry::Spread(expr) => write!(f, "...{expr}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Array {
    pub items: Vec<ArrayEntry>,
}

impl fmt::Display for Array {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, entry) in self.items.iter().enumerate() {
            if i == self.items.len() - 1 {
                write!(f, "{entry}")?
            } else {
                write!(f, "{entry}, ")?
            }
        }
        write!(f, "]")
    }
}

impl Array {
    pub(crate) fn as_const(&self) -> Option<Value> {
        let mut res = Vec::with_capacity(self.items.len());
        for entry in &self.items {
            match entry {
                ArrayEntry::Item(Expression::Const(v)) => res.push(v.node().clone()),
                _ => return None,
            }
        }
        Some(Value::from(res))
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Test {
    pub expr: Expression,
    pub name: String,
    pub kwargs: HashMap<String, Expression>,
}

impl fmt::Display for Test {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(is {}", self.expr)?;
        write!(f, " {}", self.name)?;
        write!(f, "{{",)?;

        let mut keys = self.kwargs.keys().collect::<Vec<_>>();
        keys.sort();
        for (i, k) in keys.iter().enumerate() {
            if i == self.kwargs.len() - 1 {
                write!(f, "{}={}", k, self.kwargs[*k])?
            } else {
                write!(f, "{}={}, ", k, self.kwargs[*k])?
            }
        }

        write!(f, "}})",)?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentCall {
    pub name: String,
    pub kwargs: Vec<MapEntry>,
    pub body: Vec<Node>,
    pub self_closing: bool,
}

impl fmt::Display for ComponentCall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}", self.name)?;
        write!(f, "{{",)?;
        for (i, entry) in self.kwargs.iter().enumerate() {
            if i == self.kwargs.len() - 1 {
                write!(f, "{entry}")?
            } else {
                write!(f, "{entry}, ")?
            }
        }
        write!(f, "}}",)?;

        if self.self_closing {
            write!(f, "/>")?;
        } else {
            write!(f, ">")?;
            write!(f, "[",)?;
            for node in &self.body {
                write!(f, "{:?}", node)?;
            }
            write!(f, "]",)?;

            write!(f, "<{}/>", self.name)?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionCall {
    pub name: String,
    pub kwargs: HashMap<String, Expression>,
}

impl fmt::Display for FunctionCall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        write!(f, "{{",)?;
        let mut keys = self.kwargs.keys().collect::<Vec<_>>();
        keys.sort();
        for (i, k) in keys.iter().enumerate() {
            if i == self.kwargs.len() - 1 {
                write!(f, "{}={}", k, self.kwargs[*k])?
            } else {
                write!(f, "{}={}, ", k, self.kwargs[*k])?
            }
        }
        write!(f, "}}",)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ternary {
    pub expr: Expression,
    pub true_expr: Expression,
    pub false_expr: Expression,
}

impl fmt::Display for Ternary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} if {} else {}",
            self.true_expr, self.expr, self.false_expr
        )
    }
}

/// A variable lookup
#[derive(Clone, Debug, PartialEq)]
pub struct Var {
    pub name: String,
}

impl fmt::Display for Var {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// An attribute lookup expression.
#[derive(Clone, Debug, PartialEq)]
pub struct GetAttr {
    pub expr: Expression,
    pub name: String,
    pub optional: bool,
}

impl fmt::Display for GetAttr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.optional {
            write!(f, "{}?.{}", self.expr, self.name)
        } else {
            write!(f, "{}.{}", self.expr, self.name)
        }
    }
}

/// A slicing expression (eg [-1], [1:], [:2] etc)
#[derive(Clone, Debug, PartialEq)]
pub struct Slice {
    pub expr: Expression,
    pub start: Option<Expression>,
    pub end: Option<Expression>,
    pub step: Option<Expression>,
    pub optional: bool,
}

impl fmt::Display for Slice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.optional {
            write!(f, "{}?[", self.expr)?;
        } else {
            write!(f, "{}[", self.expr)?;
        }
        if let Some(ref expr) = self.start {
            write!(f, "{}", expr)?;
        }
        if let Some(ref expr) = self.end {
            write!(f, ":{}", expr)?;
        }
        if let Some(ref expr) = self.step {
            write!(f, ":{}", expr)?;
        }
        write!(f, "]")
    }
}

/// An item lookup expression.
#[derive(Clone, Debug, PartialEq)]
pub struct GetItem {
    pub expr: Expression,
    pub sub_expr: Expression,
    pub optional: bool,
}

impl fmt::Display for GetItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.optional {
            write!(f, "{}?[{}]", self.expr, self.sub_expr)
        } else {
            write!(f, "{}[{}]", self.expr, self.sub_expr)
        }
    }
}

/// Set a variable in the context `{% set val = "hey" %}`
#[derive(Clone, Debug, PartialEq)]
pub struct Set {
    /// The name for that value in the context
    pub name: String,
    /// The value to assign
    pub value: Expression,
    /// Whether we want to set the variable globally or locally
    /// set_global is only useful in loops
    pub global: bool,
}

/// Set a variable in the context from a block `{% set val %}Hello {{world}}{% endset %}`
#[derive(Clone, Debug, PartialEq)]
pub struct BlockSet {
    /// The name for that value in the context
    pub name: String,
    /// The filters to apply to the block, with a dummy source set to null
    pub filters: Vec<Expression>,
    /// The content of the block
    pub body: Vec<Node>,
    /// Whether we want to set the variable globally or locally
    /// set_global is only useful in loops
    pub global: bool,
}

/// A template to include
#[derive(Clone, Debug, PartialEq)]
pub struct Include {
    pub name: Spanned<String>,
}

/// A block definition
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    /// The block name
    pub name: Spanned<String>,
    /// The block content
    pub body: Vec<Node>,
}

/// An if/elif/else condition with their respective body
#[derive(Clone, Debug, PartialEq)]
pub struct If {
    pub expr: Expression,
    /// The body to render in if the expr is truthy
    pub body: Vec<Node>,
    /// The body to render in if the expr is not truthy.
    /// Will also contain the elifs
    pub false_body: Vec<Node>,
}

/// A filter section node `{% filter name(param="value") %} content {% endfilter %}`
#[derive(Clone, Debug, PartialEq)]
pub struct FilterSection {
    pub name: Spanned<String>,
    pub kwargs: HashMap<String, Expression>,
    /// The filter body
    pub body: Vec<Node>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Type {
    String,
    Bool,
    Integer,
    Float,
    Number,
    Array,
    Map,
    Bytes,
}

impl FromStr for Type {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "string" => Ok(Type::String),
            "bool" => Ok(Type::Bool),
            "integer" => Ok(Type::Integer),
            "float" => Ok(Type::Float),
            "number" => Ok(Type::Number),
            "array" => Ok(Type::Array),
            "map" => Ok(Type::Map),
            "bytes" => Ok(Type::Bytes),
            _ => Err(Error::message(format!(
                "Found {s} but the only types allowed are: string, bool, integer, float, number, array, map and bytes"
            ))),
        }
    }
}

impl Type {
    pub fn as_str(&self) -> &'static str {
        match self {
            Type::String => "string",
            Type::Bool => "bool",
            Type::Integer => "integer",
            Type::Float => "float",
            Type::Number => "number",
            Type::Array => "array",
            Type::Map => "map",
            Type::Bytes => "bytes",
        }
    }

    #[inline]
    pub fn matches_value(&self, value: &Value) -> bool {
        use crate::value::ValueKind;
        match self {
            Type::String => value.is_string(),
            Type::Bool => value.is_bool(),
            Type::Integer => matches!(
                value.kind(),
                ValueKind::I64 | ValueKind::U64 | ValueKind::I128 | ValueKind::U128
            ),
            Type::Float => matches!(value.kind(), ValueKind::F64),
            Type::Number => value.is_number(),
            Type::Map => value.is_map(),
            Type::Array => value.is_array(),
            Type::Bytes => value.is_bytes(),
        }
    }

    /// Try to infer a type from a Value. When there is no equivalent, it returns None.
    pub fn from_value(val: &Value) -> Option<Self> {
        use crate::value::ValueKind;
        match val.kind() {
            ValueKind::String => Some(Type::String),
            ValueKind::Bool => Some(Type::Bool),
            ValueKind::I64 | ValueKind::I128 | ValueKind::U64 | ValueKind::U128 => {
                Some(Type::Integer)
            }
            ValueKind::F64 => Some(Type::Float),
            ValueKind::Array => Some(Type::Array),
            ValueKind::Map => Some(Type::Map),
            ValueKind::Bytes => Some(Type::Bytes),
            ValueKind::Undefined | ValueKind::None => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ComponentArgument {
    pub default: Option<Value>,
    pub typ: Option<Type>,
}

impl ComponentArgument {
    #[inline]
    pub fn type_matches(&self, value: &Value) -> bool {
        self.typ.map(|t| t.matches_value(value)).unwrap_or(true)
    }
}

/// A component definition `{% component hello() %}...{% endcomponent %}`
/// Not present in the AST, we extract them during parsing
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ComponentDefinition {
    pub name: String,
    /// The args for that component: name -> optional default value
    /// Expression for default args can only be literals
    pub kwargs: BTreeMap<String, ComponentArgument>,
    /// Rest parameter name (e.g., `...rest` collects extra kwargs into `rest`)
    /// If None, unknown kwargs will error.
    pub rest_param_name: Option<String>,
    /// Component metadata that you might need at compile time
    pub metadata: BTreeMap<String, Value>,
    pub body: Vec<Node>,
}

impl ComponentDefinition {
    pub fn kwargs_list(&self) -> Vec<&str> {
        self.kwargs.keys().map(|k| k.as_str()).collect()
    }

    /// Builds a validated context from provided kwargs, checking types and applying defaults.
    /// If rest_param_name is defined, unknown kwargs are collected into it.
    /// Otherwise, unknown kwargs will error.
    pub fn build_context<'a>(
        &self,
        provided_keys: impl Iterator<Item = &'a str>,
        get_value: impl Fn(&str) -> Option<Value>,
        body: Option<Value>,
    ) -> Result<crate::Context, String> {
        let mut context = crate::Context::new();
        let mut rest_map = crate::value::Map::new();
        let mut unknown_keys = HashSet::new();

        // Process all provided keys - collect unknowns into rest or track for error
        for key in provided_keys {
            if !self.kwargs.contains_key(key) {
                if self.rest_param_name.is_some() {
                    if let Some(value) = get_value(key) {
                        rest_map.insert(Key::from(key.to_string()), value);
                    } else {
                        unreachable!("that shouldn't be possible to get a kwarg without a value")
                    }
                } else {
                    unknown_keys.insert(key.to_string());
                }
            }
        }

        if !unknown_keys.is_empty() {
            let kwargs_list = self.kwargs_list();
            let kwargs_msg = if kwargs_list.is_empty() {
                String::new()
            } else {
                format!(
                    " Possible argument(s) are: {}",
                    kwargs_list
                        .iter()
                        .map(|s| format!("`{s}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            let unknown_list = unknown_keys
                .iter()
                .map(|s| format!("`{s}`"))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(format!(
                "Unknown argument(s) {unknown_list} in component call.{kwargs_msg}"
            ));
        }

        // Validate and apply each expected argument
        for (key, arg_def) in &self.kwargs {
            match get_value(key) {
                Some(value) => {
                    if !arg_def.type_matches(&value) {
                        return Err(format!(
                            "Component argument `{key}` (type: `{}`) does not match expected type: `{}`",
                            value.name(),
                            arg_def.typ.unwrap().as_str()
                        ));
                    }
                    context.insert_value(key.clone(), value);
                }
                None => match &arg_def.default {
                    Some(default_value) => {
                        context.insert_value(key.clone(), default_value.clone());
                    }
                    None => {
                        let typ_msg = arg_def
                            .typ
                            .map(|t| format!(" (type: `{}`)", t.as_str()))
                            .unwrap_or_default();
                        return Err(format!("Argument `{key}`{typ_msg} missing."));
                    }
                },
            }
        }

        // Add rest param if defined
        if let Some(ref rest_name) = self.rest_param_name {
            context.insert_value(rest_name.clone(), Value::from(rest_map));
        }

        // Add body if provided
        if let Some(body_value) = body {
            context.insert_value("body", body_value);
        }

        Ok(context)
    }
}

/// A forloop: can be over values or key/values
#[derive(Clone, Debug, PartialEq)]
pub struct ForLoop {
    /// Name of the key in the loop (only when iterating on map-like objects)
    pub key: Option<String>,
    /// Name of the local variable for the value in the loop
    pub value: String,
    /// Expression being iterated on
    pub target: Expression,
    /// What's in the forloop itself
    pub body: Vec<Node>,
    /// The body to execute in case of an empty object in the `{% for .. %}{% else %}{% endfor %}` construct
    pub else_body: Vec<Node>,
}

/// A `[id | str for id in ids if id > 0]` construct like in Python
/// We do not allow multiple for clause in the same list comprehension since they are confusing and probably
/// not needed in a template engine.
/// Instead of `[x for x in xs for xs in ys]` you can do `[x for x in [y for y in ys]]`
#[derive(Clone, Debug, PartialEq)]
pub struct ListComprehension {
    /// The `id | str` part in the example
    pub expr: Expression,
    /// Name of the key in the loop (only when iterating on map-like objects)
    pub key: Option<String>,
    /// Name of the local variable for the value in the loop
    pub value: String,
    /// The `ids` in the example: what we are iterating on
    pub target: Expression,
    /// The `id > 0` part in the example
    pub condition: Option<Expression>,
}

impl fmt::Display for ListComprehension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = if let Some(key) = &self.key {
            format!("{key}, {}", self.value)
        } else {
            self.value.to_string()
        };

        let cond = if let Some(condition) = &self.condition {
            format!(" if {condition}")
        } else {
            String::new()
        };
        write!(f, "[{} for {names} in {}{cond}]", self.expr, self.target)
    }
}

#[derive(Clone, PartialEq)]
pub enum Node {
    Content(String),
    Expression(Expression),
    Set(Set),
    BlockSet(BlockSet),
    Include(Include),
    Block(Block),
    ForLoop(ForLoop),
    Break,
    Continue,
    If(If),
    FilterSection(FilterSection),
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Node::*;

        match self {
            Content(s) => fmt::Debug::fmt(s, f),
            Expression(s) => fmt::Debug::fmt(s, f),
            Set(s) => fmt::Debug::fmt(s, f),
            BlockSet(s) => fmt::Debug::fmt(s, f),
            Include(s) => fmt::Debug::fmt(s, f),
            Block(s) => fmt::Debug::fmt(s, f),
            ForLoop(s) => fmt::Debug::fmt(s, f),
            If(s) => fmt::Debug::fmt(s, f),
            FilterSection(s) => fmt::Debug::fmt(s, f),
            Break => fmt::Debug::fmt("{% break %}", f),
            Continue => fmt::Debug::fmt("{% continue %}", f),
        }
    }
}
