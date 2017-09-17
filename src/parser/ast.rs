use std::collections::HashMap;
use std::fmt;


/// Whether to remove the whitespace of a `{% %}` tag
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WS {
    /// `true` if the tag is `{%-`
    pub left: bool,
    /// `true` if the tag is `-%}`
    pub right: bool,
}

impl Default for WS {
    fn default() -> Self {
        WS {
            left: false,
            right: false,
        }
    }
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
}

impl fmt::Display for MathOperator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            MathOperator::Add => "+",
            MathOperator::Sub => "-",
            MathOperator::Mul => "*",
            MathOperator::Div => "/",
        })
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
        write!(f, "{}", match *self {
            LogicOperator::Gt => ">",
            LogicOperator::Gte => ">=",
            LogicOperator::Lt => "<",
            LogicOperator::Lte => "<=",
            LogicOperator::Eq => "==",
            LogicOperator::NotEq => "!=",
            LogicOperator::And => "and",
            LogicOperator::Or => "or",
        })
    }
}

/// A function call, can be a filter or a global function
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionCall {
    pub name: String,
    pub args: HashMap<String, Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Ident {
    pub name: String,
    pub filters: Vec<FunctionCall>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MathExpr {
    pub lhs: Box<Expr>,
    pub rhs: Box<Expr>,
    pub operator: MathOperator,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogicExpr {
    pub lhs: Box<Expr>,
    pub rhs: Box<Expr>,
    pub operator: LogicOperator,
}

/// An expression is the node found in variable block, kwargs and conditions.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Ident(Ident),
    Math(MathExpr),
    Logic(LogicExpr),
    Test(Test),
    MacroCall(MacroCall),
    FunctionCall(FunctionCall),
    // A negated expression is still an expression!
    Not(Box<Expr>),
}

/// A test node `if my_var is odd`
#[derive(Clone, Debug, PartialEq)]
pub struct Test {
    /// Which expression is evaluated
    pub ident: Ident,
    /// Name of the test
    pub name: String,
    /// Any optional arg given to the test
    pub args: Vec<Expr>,
}


/// A filter section node `{{ filter name(param="value") }} content {{ endfilter }}`
#[derive(Clone, Debug, PartialEq)]
pub struct FilterSection {
    pub filter: FunctionCall,
    pub body: Vec<Node>,
}

/// Set a variable in the context `{% set val = "hey" %}`
#[derive(Clone, Debug, PartialEq)]
pub struct Set {
    /// The name for that value in the context
    pub key: String,
    pub value: Expr,
}

/// A call to a namespaced macro `macros::my_macro()`
#[derive(Clone, Debug, PartialEq)]
pub struct MacroCall {
    pub namespace: String,
    pub name: String,
    pub args: HashMap<String, Expr>,
}


#[derive(Clone, Debug, PartialEq)]
pub struct MacroDefinition {
    pub name: String,
    /// The args for that macro: name -> optional default value
    pub args: HashMap<String, Option<Expr>>,
    pub body: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub name: String,
    pub body: Vec<Node>,
}

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
}


/// All Tera nodes that can be encountered
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// A call to `{{ super() }}` in a block
    Super,
    /// Some actual text
    Text(String),
    /// The text between `{% raw %}` and `{% endraw %}`
    Raw(WS, String, WS),
    /// A filter section node `{{ filter name(param="value") }} content {{ endfilter }}`
    FilterSection(WS, FilterSection, WS),
    /// The `{% extends "blabla.html" %}` node, contains the template name
    Extends(WS, String),
    /// The `{% include "blabla.html" %}` node, contains the template name
    Include(WS, String),
    /// The `{% set val = something %}` tag
    Set(WS, Set),
    /// The {% import "macros.html" as macros %}
    ImportMacro(WS, String, String),
    /// The full template AST
    Template(Vec<Node>),
    /// A `{{ }}` block
    VariableBlock(Expr),
    /// A `{% block name %}...{% endblock %}`
    Block(WS, Block, WS),
    /// A `{% macro hello() %}...{% endmacro %}`
    MacroDefinition(MacroDefinition),
    /// A `{% for i in items %}...{% endfor %}`
    Forloop(WS, Forloop, WS),
}

pub type Template = Vec<Node>;
