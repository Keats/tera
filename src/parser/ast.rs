use std::collections::HashMap;
use std::fmt;

/// Whether to remove the whitespace of a `{% %}` tag
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct WS {
    /// `true` if the tag is `{%-`
    pub left: bool,
    /// `true` if the tag is `-%}`
    pub right: bool,
}

/// All math operators
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MathOperator {
    /// +
    Add,
    /// -
    Sub,
    /// *
    Mul,
    /// /
    Div,
    /// %
    Modulo,
}

impl fmt::Display for MathOperator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                MathOperator::Add => "+",
                MathOperator::Sub => "-",
                MathOperator::Mul => "*",
                MathOperator::Div => "/",
                MathOperator::Modulo => "%",
            }
        )
    }
}

/// All logic operators
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LogicOperator {
    /// >
    Gt,
    /// >=
    Gte,
    /// <
    Lt,
    /// <=
    Lte,
    /// ==
    Eq,
    /// !=
    NotEq,
    /// and
    And,
    /// or
    Or,
}

impl fmt::Display for LogicOperator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                LogicOperator::Gt => ">",
                LogicOperator::Gte => ">=",
                LogicOperator::Lt => "<",
                LogicOperator::Lte => "<=",
                LogicOperator::Eq => "==",
                LogicOperator::NotEq => "!=",
                LogicOperator::And => "and",
                LogicOperator::Or => "or",
            }
        )
    }
}

/// A function call, can be a filter or a global function
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionCall {
    /// The name of the function
    pub name: String,
    /// The args of the function: key -> value
    pub args: HashMap<String, Expr>,
}

/// A mathematical expression
#[derive(Clone, Debug, PartialEq)]
pub struct MathExpr {
    /// The left hand side of the expression
    pub lhs: Box<Expr>,
    /// The right hand side of the expression
    pub rhs: Box<Expr>,
    /// The operator used
    pub operator: MathOperator,
}

/// A logical expression
#[derive(Clone, Debug, PartialEq)]
pub struct LogicExpr {
    /// The left hand side of the expression
    pub lhs: Box<Expr>,
    /// The right hand side of the expression
    pub rhs: Box<Expr>,
    /// The operator used
    pub operator: LogicOperator,
}

/// Can only be a combination of string + ident or ident + ident
#[derive(Clone, Debug, PartialEq)]
pub struct StringConcat {
    /// All the values we're concatening into a string
    pub values: Vec<ExprVal>,
}

impl StringConcat {
    pub(crate) fn to_template_string(&self) -> String {
        let mut res = Vec::new();
        for value in &self.values {
            match value {
                ExprVal::String(ref s) => res.push(format!("'{}'", s)),
                ExprVal::Ident(ref s) => res.push(s.to_string()),
                _ => res.push("unknown".to_string()),
            }
        }

        res.join(" ~ ")
    }
}

/// Something that checks whether the left side is contained in the right side
#[derive(Clone, Debug, PartialEq)]
pub struct In {
    /// The needle, a string or a basic expression/literal
    pub lhs: Box<Expr>,
    /// The haystack, can be a string, an array or an ident only currently
    pub rhs: Box<Expr>,
    /// Is it using `not` as in `b` not in `...`?
    pub negated: bool,
}

/// An expression is the node found in variable block, kwargs and conditions.
#[derive(Clone, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum ExprVal {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Ident(String),
    Math(MathExpr),
    Logic(LogicExpr),
    Test(Test),
    MacroCall(MacroCall),
    FunctionCall(FunctionCall),
    // A vec of Expr, not ExprVal since filters are allowed
    // on values inside arrays
    Array(Vec<Expr>),
    StringConcat(StringConcat),
    In(In),
}

/// An expression is a value that can be negated and followed by
/// optional filters
#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    /// The expression we are evaluating
    pub val: ExprVal,
    /// Is it using `not`?
    pub negated: bool,
    /// List of filters used on that value
    pub filters: Vec<FunctionCall>,
}

impl Expr {
    /// Create a new basic Expr
    pub fn new(val: ExprVal) -> Expr {
        Expr { val, negated: false, filters: vec![] }
    }

    /// Create a new negated Expr
    pub fn new_negated(val: ExprVal) -> Expr {
        Expr { val, negated: true, filters: vec![] }
    }

    /// Create a new basic Expr with some filters
    pub fn with_filters(val: ExprVal, filters: Vec<FunctionCall>) -> Expr {
        Expr { val, filters, negated: false }
    }

    /// Check if the expr has a default filter as first filter
    pub fn has_default_filter(&self) -> bool {
        if self.filters.is_empty() {
            return false;
        }

        self.filters[0].name == "default"
    }

    /// Check if the last filter is `safe`
    pub fn is_marked_safe(&self) -> bool {
        if self.filters.is_empty() {
            return false;
        }

        self.filters[self.filters.len() - 1].name == "safe"
    }
}

/// A test node `if my_var is odd`
#[derive(Clone, Debug, PartialEq)]
pub struct Test {
    /// Which variable is evaluated
    pub ident: String,
    /// Is it using `not`?
    pub negated: bool,
    /// Name of the test
    pub name: String,
    /// Any optional arg given to the test
    pub args: Vec<Expr>,
}

/// A filter section node `{{ filter name(param="value") }} content {{ endfilter }}`
#[derive(Clone, Debug, PartialEq)]
pub struct FilterSection {
    /// The filter call itsel
    pub filter: FunctionCall,
    /// The filter body
    pub body: Vec<Node>,
}

/// Set a variable in the context `{% set val = "hey" %}`
#[derive(Clone, Debug, PartialEq)]
pub struct Set {
    /// The name for that value in the context
    pub key: String,
    /// The value to assign
    pub value: Expr,
    /// Whether we want to set the variable globally or locally
    /// global_set is only useful in loops
    pub global: bool,
}

/// A call to a namespaced macro `macros::my_macro()`
#[derive(Clone, Debug, PartialEq)]
pub struct MacroCall {
    /// The namespace we're looking for that macro in
    pub namespace: String,
    /// The macro name
    pub name: String,
    /// The args for that macro: name -> value
    pub args: HashMap<String, Expr>,
}

/// A Macro definition
#[derive(Clone, Debug, PartialEq)]
pub struct MacroDefinition {
    /// The macro name
    pub name: String,
    /// The args for that macro: name -> optional default value
    pub args: HashMap<String, Option<Expr>>,
    /// The macro content
    pub body: Vec<Node>,
}

/// A block definition
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    /// The block name
    pub name: String,
    /// The block content
    pub body: Vec<Node>,
}

/// A forloop: can be over values or key/values
#[derive(Clone, Debug, PartialEq)]
pub struct Forloop {
    /// Name of the key in the loop (only when iterating on map-like objects)
    pub key: Option<String>,
    /// Name of the local variable for the value in the loop
    pub value: String,
    /// Expression being iterated on
    pub container: Expr,
    /// What's in the forloop itself
    pub body: Vec<Node>,
    /// The body to execute in case of an empty object
    pub empty_body: Option<Vec<Node>>,
}

/// An if/elif/else condition with their respective body
#[derive(Clone, Debug, PartialEq)]
pub struct If {
    /// First item if the if, all the ones after are elif
    pub conditions: Vec<(WS, Expr, Vec<Node>)>,
    /// The optional `else` block
    pub otherwise: Option<(WS, Vec<Node>)>,
}

/// All Tera nodes that can be encountered
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// A call to `{{ super() }}` in a block
    Super,

    /// Some actual text
    Text(String),
    /// A `{{ }}` block
    VariableBlock(WS, Expr),
    /// A `{% macro hello() %}...{% endmacro %}`
    MacroDefinition(WS, MacroDefinition, WS),

    /// The `{% extends "blabla.html" %}` node, contains the template name
    Extends(WS, String),
    /// The `{% include "blabla.html" %}` node, contains the template name
    Include(WS, Vec<String>, bool),
    /// The `{% import "macros.html" as macros %}`
    ImportMacro(WS, String, String),
    /// The `{% set val = something %}` tag
    Set(WS, Set),

    /// The text between `{% raw %}` and `{% endraw %}`
    Raw(WS, String, WS),

    /// A filter section node `{{ filter name(param="value") }} content {{ endfilter }}`
    FilterSection(WS, FilterSection, WS),
    /// A `{% block name %}...{% endblock %}`
    Block(WS, Block, WS),
    /// A `{% for i in items %}...{% endfor %}`
    Forloop(WS, Forloop, WS),

    /// A if/elif/else block, WS for the if/elif/else is directly in the struct
    If(If, WS),

    /// The `{% break %}` tag
    Break(WS),
    /// The `{% continue %}` tag
    Continue(WS),

    /// The `{# #} `comment tag and its content
    Comment(WS, String),
}
