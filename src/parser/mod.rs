use std::collections::HashMap;

use pest::{Parser, Error as PestError};
use pest::prec_climber::{PrecClimber, Operator, Assoc};
use pest::iterators::Pair;

use errors::{Result as TeraResult, ResultExt};

// This include forces recompiling this source file if the grammar file changes.
// Uncomment it when doing changes to the .pest file
const _GRAMMAR: &'static str = include_str!("tera.pest");


#[derive(Parser)]
#[grammar = "parser/tera.pest"]
pub struct TeraParser;

pub mod ast;
mod whitespace;

#[cfg(test)]
mod tests;

pub use self::whitespace::remove_whitespace;
use self::ast::*;

lazy_static! {
    static ref MATH_CLIMBER: PrecClimber<Rule> = PrecClimber::new(vec![
        // +, -
        Operator::new(Rule::op_plus, Assoc::Left) | Operator::new(Rule::op_minus, Assoc::Left),
        // *, /, %
        Operator::new(Rule::op_times, Assoc::Left) |
        Operator::new(Rule::op_slash, Assoc::Left) |
        Operator::new(Rule::op_modulo, Assoc::Left) |
        Operator::new(Rule::op_tilde, Assoc::Left),
    ]);
    static ref COMPARISON_EXPR_CLIMBER: PrecClimber<Rule> = PrecClimber::new(vec![
        // <, <=, >, >=, ==, !=
        Operator::new(Rule::op_lt, Assoc::Left) | Operator::new(Rule::op_lte, Assoc::Left)
        | Operator::new(Rule::op_gt, Assoc::Left) | Operator::new(Rule::op_gte, Assoc::Left)
        | Operator::new(Rule::op_eq, Assoc::Left) | Operator::new(Rule::op_ineq, Assoc::Left),
    ]);
    static ref LOGIC_EXPR_CLIMBER: PrecClimber<Rule> = PrecClimber::new(vec![
        Operator::new(Rule::op_or, Assoc::Left),
        Operator::new(Rule::op_and, Assoc::Left),
    ]);
}

fn parse_kwarg(pair: Pair<Rule>) -> (String, Expr) {
    let mut name = None;
    let mut val = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::ident => name = Some(p.into_span().as_str().to_string()),
            Rule::logic_expr => val = Some(parse_logic_expr(p)),
            _ => unreachable!("{:?} not supposed to get there (parse_kwarg)!", p.as_rule())
        };
    }

    (name.unwrap(), val.unwrap())
}

fn parse_fn_call(pair: Pair<Rule>) -> FunctionCall {
    let mut name = None;
    let mut args = HashMap::new();

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::ident => name = Some(p.into_span().as_str().to_string()),
            Rule::kwarg => {
                let (name, val) = parse_kwarg(p);
                args.insert(name, val);
            }
            _ => unreachable!("{:?} not supposed to get there (parse_fn_call)!", p.as_rule())
        };
    }

    FunctionCall { name: name.unwrap(), args }
}

fn parse_filter(pair: Pair<Rule>) -> FunctionCall {
    let mut name = None;
    let mut args = HashMap::new();
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::ident => name = Some(p.into_span().as_str().to_string()),
            Rule::kwarg => {
                let (name, val) = parse_kwarg(p);
                args.insert(name, val);
            }
            Rule::fn_call => {
                return parse_fn_call(p);
            }
            _ => unreachable!("{:?} not supposed to get there (parse_filter)!", p.as_rule())
        };
    }

    FunctionCall { name: name.unwrap(), args }
}

fn parse_test_call(pair: Pair<Rule>) -> (String, Vec<Expr>) {
    let mut name = None;
    let mut args = vec![];

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::ident => name = Some(p.into_span().as_str().to_string()),
            Rule::test_args =>
                // iterate on the test_arg rule
                for p2 in p.into_inner() {
                    // only expressions allowed in the grammar so we skip the
                    // matching
                    for p3 in p2.into_inner() {
                        args.push(parse_logic_expr(p3));
                    }
                },
            _ => unreachable!("{:?} not supposed to get there (parse_test_call)!", p.as_rule())
        };
    }

    (name.unwrap(), args)
}

fn parse_test(pair: Pair<Rule>) -> Test {
    let mut ident = None;
    let mut name = None;
    let mut args = vec![];

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::dotted_ident => ident = Some(p.as_str().to_string()),
            Rule::test_call => {
                let (_name, _args) = parse_test_call(p);
                name = Some(_name);
                args = _args;
            },
            _ => unreachable!("{:?} not supposed to get there (parse_ident)!", p.as_rule())
        };
    }

    Test { ident: ident.unwrap(), name: name.unwrap(), args }

}

fn parse_basic_expression(pair: Pair<Rule>) -> ExprVal {
    let primary = |pair| {
        parse_basic_expression(pair)
    };

    let infix = |lhs: ExprVal, op: Pair<Rule>, rhs: ExprVal| {
        ExprVal::Math(
            MathExpr {
                lhs: Box::new(Expr::new(lhs)),
                operator: match op.as_rule() {
                    Rule::op_plus => MathOperator::Add,
                    Rule::op_minus => MathOperator::Sub,
                    Rule::op_times => MathOperator::Mul,
                    Rule::op_slash => MathOperator::Div,
                    Rule::op_modulo => MathOperator::Modulo,
                    Rule::op_tilde => MathOperator::Tilde,
                    _ => unreachable!()
                },
                rhs: Box::new(Expr::new(rhs)),
            }
        )
    };

    match pair.as_rule() {
        Rule::int => ExprVal::Int(pair.as_str().parse().unwrap()),
        Rule::float => ExprVal::Float(pair.as_str().parse().unwrap()),
        Rule::boolean => match pair.as_str() {
            "true" => ExprVal::Bool(true),
            "false" => ExprVal::Bool(false),
            _ => unreachable!(),
        },
        Rule::test => ExprVal::Test(parse_test(pair)),
        Rule::fn_call => ExprVal::FunctionCall(parse_fn_call(pair)),
        Rule::macro_call => ExprVal::MacroCall(parse_macro_call(pair)),
        Rule::string => ExprVal::String(pair.as_str().replace("\"", "").to_string()),
        Rule::dotted_ident => ExprVal::Ident(pair.as_str().to_string()),
        Rule::basic_expr => MATH_CLIMBER.climb(pair.into_inner(), primary, infix),
        _ => unreachable!("Got {:?} in parse_basic_expression", pair.as_rule())
    }
}

/// A basic expression with optional filters
fn parse_basic_expr_with_filters(pair: Pair<Rule>) -> Expr {
    let mut expr = None;
    let mut filters = vec![];

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::basic_expr => expr = Some(parse_basic_expression(p)),
            Rule::filter => filters.push(parse_filter(p)),
            _ => unreachable!("Got {:?}", p),
        };
    }

    Expr { val: expr.unwrap(), negated: false, filters }
}


/// A basic expression with optional filters
/// TODO: to rewrite
fn parse_comparison_val(pair: Pair<Rule>) -> Expr {
    let primary = |pair| {
        parse_comparison_val(pair)
    };

    let infix = |lhs: Expr, op: Pair<Rule>, rhs: Expr| {
        Expr::new(ExprVal::Math(
            MathExpr {
                lhs: Box::new(lhs),
                operator: match op.as_rule() {
                    Rule::op_plus => MathOperator::Add,
                    Rule::op_minus => MathOperator::Sub,
                    Rule::op_times => MathOperator::Mul,
                    Rule::op_slash => MathOperator::Div,
                    Rule::op_modulo => MathOperator::Modulo,
                    _ => unreachable!()
                },
                rhs: Box::new(rhs),
            }
        ))
    };

    match pair.as_rule() {
        Rule::basic_expr_filter => parse_basic_expr_with_filters(pair),
        Rule::comparison_val => MATH_CLIMBER.climb(pair.into_inner(), primary, infix),
        _ => unreachable!("Got {:?} in parse_comparison_val", pair.as_rule())
    }
}

fn parse_comparison_expression(pair: Pair<Rule>) -> Expr {
    let primary = |pair| {
        parse_comparison_expression(pair)
    };

    let infix = |lhs: Expr, op: Pair<Rule>, rhs: Expr| {
        Expr::new(
            ExprVal::Logic(
                LogicExpr {
                    lhs: Box::new(lhs),
                    operator: match op.as_rule() {
                        Rule::op_lt => LogicOperator::Lt,
                        Rule::op_lte => LogicOperator::Lte,
                        Rule::op_gt => LogicOperator::Gt,
                        Rule::op_gte => LogicOperator::Gte,
                        Rule::op_ineq => LogicOperator::NotEq,
                        Rule::op_eq => LogicOperator::Eq,
                        _ => unreachable!()
                    },
                    rhs: Box::new(rhs),
                }
            )
        )
    };

    match pair.as_rule() {
        Rule::comparison_val => parse_comparison_val(pair),
        Rule::comparison_expr => COMPARISON_EXPR_CLIMBER.climb(pair.into_inner(), primary, infix),
        _ => unreachable!("Got {:?} in parse_comparison_expression", pair.as_rule())
    }
}

/// An expression that can be negated
fn parse_logic_val(pair: Pair<Rule>) -> Expr {
    let mut negated = false;
    let mut expr = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::op_not => negated = true,
            Rule::comparison_expr => expr = Some(parse_comparison_expression(p)),
            _=> unreachable!(),
        };
    }

    let mut e = expr.unwrap();
    e.negated = negated;
    e
}

fn parse_logic_expr(pair: Pair<Rule>) -> Expr {
    let primary = |pair: Pair<Rule>| {
        parse_logic_expr(pair)
    };

    let infix = |lhs: Expr, op: Pair<Rule>, rhs: Expr| {
        match op.as_rule() {
            Rule::op_or => {
                Expr::new(ExprVal::Logic(LogicExpr {
                    lhs: Box::new(lhs),
                    operator: LogicOperator::Or,
                    rhs: Box::new(rhs)
                }))
            }
            Rule::op_and => {
                Expr::new(ExprVal::Logic(LogicExpr {
                    lhs: Box::new(lhs),
                    operator: LogicOperator::And,
                    rhs: Box::new(rhs)
                }))
            }
            _ => unreachable!("{:?} not supposed to get there (infix of logic_expression)!", op.as_rule())
        }
    };

    match pair.as_rule() {
        Rule::logic_val => parse_logic_val(pair),
        Rule::logic_expr => LOGIC_EXPR_CLIMBER.climb(pair.into_inner(), primary, infix),
        _ => unreachable!("Got {:?} in parse_logic_expr", pair.as_rule())
    }
}

fn parse_macro_call(pair: Pair<Rule>) -> MacroCall {
    let mut namespace = None;
    let mut name = None;
    let mut args = HashMap::new();

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::ident => {
                // namespace comes first
                if namespace.is_none() {
                    namespace = Some(p.into_span().as_str().to_string());
                } else {
                    name = Some(p.into_span().as_str().to_string());
                }
            },
            Rule::kwarg => {
                let (key, val) = parse_kwarg(p);
                args.insert(key, val);
            },
            _ => unreachable!("Got {:?} in parse_macro_call", p.as_rule())

        }
    }

    MacroCall { namespace: namespace.unwrap(), name: name.unwrap(), args }
}

fn parse_variable_tag(pair: Pair<Rule>) -> Node {
    let p = pair.into_inner().nth(0).unwrap();
    Node::VariableBlock(parse_logic_expr(p))
}

fn parse_import_macro(pair: Pair<Rule>) -> Node {
    let mut ws = WS::default();
    let mut file = None;
    let mut ident = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::tag_start => {
                ws.left = p.into_span().as_str() == "{%-";
            }
            Rule::string => file = Some(p.into_span().as_str().replace("\"", "").to_string()),
            Rule::ident => ident = Some(p.into_span().as_str().to_string()),
            Rule::tag_end => {
                ws.right = p.into_span().as_str() == "-%}";
            }
            _ => unreachable!()
        };
    }

    Node::ImportMacro(ws, file.unwrap(), ident.unwrap())
}

/// `extends` and `include` have the same structure so only way fn to parse them both
fn parse_extends_include(pair: Pair<Rule>) -> (WS, String) {
    let mut ws = WS::default();
    let mut file = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::tag_start => {
                ws.left = p.into_span().as_str() == "{%-";
            }
            Rule::string => file = Some(p.into_span().as_str().replace("\"", "").to_string()),
            Rule::tag_end => {
                ws.right = p.into_span().as_str() == "-%}";
            }
            _ => unreachable!()
        };
    }

    (ws, file.unwrap())
}

fn parse_set_tag(pair: Pair<Rule>) -> Node {
    let mut ws = WS::default();
    let mut key = None;
    let mut expr = None;
    let mut global = false;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::tag_start => {
                ws.left = p.into_span().as_str() == "{%-";
            },
            Rule::tag_end => {
                ws.right = p.into_span().as_str() == "-%}";
            },
            Rule::set_scope => {
                global = p.into_span().as_str() == "set_global";
            }
            Rule::ident => key = Some(p.as_str().to_string()),
            Rule::logic_expr=> expr = Some(parse_logic_expr(p)),
            _ => unreachable!("unexpected {:?} rule in parse_set_tag", p.as_rule()),
        }
    }

    Node::Set(ws, Set {key: key.unwrap(), value: expr.unwrap(), global})
}

fn parse_raw_tag(pair: Pair<Rule>) -> Node {
    let mut start_ws = WS::default();
    let mut end_ws = WS::default();
    let mut text = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::raw_tag => {
                for p2 in p.into_inner() {
                    match p2.as_rule() {
                        Rule::tag_start => start_ws.left = p2.into_span().as_str() == "{%-",
                        Rule::tag_end => start_ws.right = p2.into_span().as_str() == "-%}",
                        _ => unreachable!(),
                    }
                }
            },
            Rule::raw_text => text = Some(p.as_str().to_string()),
            Rule::endraw_tag => {
                for p2 in p.into_inner() {
                    match p2.as_rule() {
                        Rule::tag_start => end_ws.left = p2.into_span().as_str() == "{%-",
                        Rule::tag_end => end_ws.right = p2.into_span().as_str() == "-%}",
                        _ => unreachable!(),
                    }
                }
            },
            _ => unreachable!("unexpected {:?} rule in parse_raw_tag", p.as_rule()),
        };
    }

    Node::Raw(start_ws, text.unwrap(), end_ws)
}

fn parse_filter_section(pair: Pair<Rule>) -> Node {
    let mut start_ws = WS::default();
    let mut end_ws = WS::default();
    let mut filter = None;
    let mut body = vec![];

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::filter_tag => {
                for p2 in p.into_inner() {
                    match p2.as_rule() {
                        Rule::tag_start => start_ws.left = p2.into_span().as_str() == "{%-",
                        Rule::tag_end => start_ws.right = p2.into_span().as_str() == "-%}",
                        Rule::fn_call => filter = Some(parse_fn_call(p2)),
                        Rule::ident => filter = Some(FunctionCall { name: p2.as_str().to_string(), args: HashMap::new() }),
                        _ => unreachable!("Got {:?} while parsing filter_tag", p2),
                    }
                }
            },
            Rule::content | Rule::macro_content | Rule::block_content => body.extend(parse_content(p)),
            Rule::endfilter_tag => {
                for p2 in p.into_inner() {
                    match p2.as_rule() {
                        Rule::tag_start => end_ws.left = p2.into_span().as_str() == "{%-",
                        Rule::tag_end => end_ws.right = p2.into_span().as_str() == "-%}",
                        _ => unreachable!(),
                    }
                }
            },
            _ => unreachable!("unexpected {:?} rule in parse_filter_section", p.as_rule()),
        };
    }

    Node::FilterSection(start_ws, FilterSection {filter: filter.unwrap(), body}, end_ws)
}

fn parse_block(pair: Pair<Rule>) -> Node {
    let mut start_ws = WS::default();
    let mut end_ws = WS::default();
    let mut name = None;
    let mut body = vec![];

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::block_tag => {
                for p2 in p.into_inner() {
                    match p2.as_rule() {
                        Rule::tag_start => start_ws.left = p2.into_span().as_str() == "{%-",
                        Rule::tag_end => start_ws.right = p2.into_span().as_str() == "-%}",
                        Rule::ident => name = Some(p2.into_span().as_str().to_string()),
                        _ => unreachable!(),
                    };
                }
            },
            Rule::block_content => body.extend(parse_content(p)),
            Rule::endblock_tag => {
                for p2 in p.into_inner() {
                    match p2.as_rule() {
                        Rule::tag_start => end_ws.left = p2.into_span().as_str() == "{%-",
                        Rule::tag_end => end_ws.right = p2.into_span().as_str() == "-%}",
                        Rule::ident => (),
                        _ => unreachable!(),
                    };
                }
            },
            _ => unreachable!("unexpected {:?} rule in parse_filter_section", p.as_rule()),
        };
    }

    Node::Block(start_ws, Block {name: name.unwrap(), body} ,end_ws)
}

fn parse_macro_definition(pair: Pair<Rule>) -> Node {
    let mut start_ws = WS::default();
    let mut end_ws = WS::default();
    let mut name = None;
    let mut args = HashMap::new();
    let mut body = vec![];

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::macro_tag => {
                for p2 in p.into_inner() {
                    match p2.as_rule() {
                        Rule::tag_start => start_ws.left = p2.into_span().as_str() == "{%-",
                        Rule::tag_end => start_ws.right = p2.into_span().as_str() == "-%}",
                        Rule::ident => name = Some(p2.as_str().to_string()),
                        Rule::macro_def_arg => {
                            let mut arg_name = None;
                            let mut default_val = None;
                            for p3 in p2.into_inner() {
                                match p3.as_rule() {
                                    Rule::ident => arg_name = Some(p3.as_str().to_string()),
                                    // no filters allowed on macro definition
                                    _ => default_val = Some(Expr::new(parse_basic_expression(p3))),
                                };
                            }
                            args.insert(arg_name.unwrap(), default_val);
                        },
                        _ => continue,
                    };
                }
            },
            Rule::macro_content => body.extend(parse_content(p)),
            Rule::endmacro_tag => {
                for p2 in p.into_inner() {
                    match p2.as_rule() {
                        Rule::tag_start => end_ws.left = p2.into_span().as_str() == "{%-",
                        Rule::tag_end => end_ws.right = p2.into_span().as_str() == "-%}",
                        Rule::ident => (),
                        _ => unreachable!(),
                    };
                }
            },
            _ => unreachable!("unexpected {:?} rule in parse_macro_definition", p.as_rule()),
        }
    }

    Node::MacroDefinition(start_ws, MacroDefinition {name: name.unwrap(), args, body}, end_ws)
}

fn parse_forloop(pair: Pair<Rule>) -> Node {
    let mut start_ws = WS::default();
    let mut end_ws = WS::default();

    let mut key = None;
    let mut value = None;
    let mut container = None;
    let mut body = vec![];

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::for_tag => {
                let mut idents = vec![];
                for p2 in p.into_inner() {
                    match p2.as_rule() {
                        Rule::tag_start => start_ws.left = p2.into_span().as_str() == "{%-",
                        Rule::tag_end => start_ws.right = p2.into_span().as_str() == "-%}",
                        Rule::ident => idents.push(p2.as_str().to_string()),
                        Rule::logic_expr => container = Some(parse_logic_expr(p2)),
                        _ => unreachable!(),
                    };
                }

                if idents.len() == 1 {
                    value = Some(idents[0].clone());
                } else {
                    key = Some(idents[0].clone());
                    value = Some(idents[1].clone());
                }
            },
            Rule::content | Rule::macro_content | Rule::block_content => body.extend(parse_content(p)),
            Rule::endfor_tag => {
                for p2 in p.into_inner() {
                    match p2.as_rule() {
                        Rule::tag_start => end_ws.left = p2.into_span().as_str() == "{%-",
                        Rule::tag_end => end_ws.right = p2.into_span().as_str() == "-%}",
                        Rule::ident => (),
                        _ => unreachable!(),
                    };
                }
            },
            _ => unreachable!("unexpected {:?} rule in parse_forloop", p.as_rule()),
        };
    }

    Node::Forloop(
        start_ws,
        Forloop {
            key,
            value: value.unwrap(),
            container: container.unwrap(),
            body,
        },
        end_ws,
    )
}

fn parse_if(pair: Pair<Rule>) -> Node {
    // the `endif` tag ws handling
    let mut end_ws = WS::default();
    let mut conditions = vec![];
    let mut otherwise = None;

    // the current node we're exploring
    let mut current_ws = WS::default();
    let mut expr = None;
    let mut current_body = vec![];
    let mut in_else = false;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::if_tag | Rule::elif_tag => {
                // Reset everything for elifs
                if p.as_rule() == Rule::elif_tag {
                    conditions.push((current_ws, expr.unwrap(), current_body));
                    expr = None;
                    current_ws = WS::default();
                    current_body = vec![];
                }

                for p2 in p.into_inner() {
                    match p2.as_rule() {
                        Rule::tag_start => current_ws.left = p2.into_span().as_str() == "{%-",
                        Rule::tag_end => current_ws.right = p2.into_span().as_str() == "-%}",
                        Rule::logic_expr => expr = Some(parse_logic_expr(p2)),
                        _ => unreachable!(),
                    };
                }
            },
            Rule::content | Rule::macro_content | Rule::block_content => current_body.extend(parse_content(p)),
            Rule::else_tag => {
                // had an elif before the else
                if expr.is_some() {
                    conditions.push((current_ws, expr.unwrap(), current_body));
                    expr = None;
                    current_ws = WS::default();
                    current_body = vec![];
                }
                in_else = true;
                for p2 in p.into_inner() {
                    match p2.as_rule() {
                        Rule::tag_start => current_ws.left = p2.into_span().as_str() == "{%-",
                        Rule::tag_end => current_ws.right = p2.into_span().as_str() == "-%}",
                        _ => unreachable!(),
                    };
                }
            },
            Rule::endif_tag => {
                if in_else {
                    otherwise = Some((current_ws, current_body));
                } else {
                    // the last elif
                    conditions.push((current_ws, expr.unwrap(), current_body));
                }

                for p2 in p.into_inner() {
                    match p2.as_rule() {
                        Rule::tag_start => end_ws.left = p2.into_span().as_str() == "{%-",
                        Rule::tag_end => end_ws.right = p2.into_span().as_str() == "-%}",
                        _ => unreachable!(),
                    };
                }
                break;
            },
            _ => unreachable!("unreachable rule in parse_if: {:?}", p.as_rule()),
        }
    }

    Node::If(If {conditions, otherwise}, end_ws)
}

fn parse_content(pair: Pair<Rule>) -> Vec<Node> {
    let mut nodes = vec![];

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::include_tag => {
                let (ws, file) = parse_extends_include(p);
                nodes.push(Node::Include(ws, file));
            },
            // Ignore comments
            Rule::comment_tag => (),
            Rule::super_tag => nodes.push(Node::Super),
            Rule::set_tag => nodes.push(parse_set_tag(p)),
            Rule::raw => nodes.push(parse_raw_tag(p)),
            Rule::variable_tag => nodes.push(parse_variable_tag(p)),
            Rule::import_macro_tag => nodes.push(parse_import_macro(p)),
            Rule::macro_definition => nodes.push(parse_macro_definition(p)),
            Rule::forloop | Rule::macro_forloop | Rule::block_forloop => nodes.push(parse_forloop(p)),
            Rule::content_if | Rule::macro_if | Rule::block_if => nodes.push(parse_if(p)),
            Rule::filter_section | Rule::macro_filter_section | Rule::block_filter_section => {
                nodes.push(parse_filter_section(p))
            },
            Rule::text => nodes.push(Node::Text(p.into_span().as_str().to_string())),
            Rule::block => nodes.push(parse_block(p)),
            _ => unreachable!("unreachable content rule: {:?}", p.as_rule())
        };
    }

    nodes
}

pub fn parse(input: &str) -> TeraResult<Vec<Node>> {
    let mut pairs = match TeraParser::parse(Rule::template, input) {
        Ok(p) => p,
        Err(e) => match e {
            PestError::ParsingError { pos, .. } => {
                let (line_no, col_no) = pos.line_col();
                bail!("Invalid Tera syntax at line {}, col {}", line_no, col_no);
            },
            _ => unreachable!(),
        }
    };


    let mut nodes = vec![];

    // We must have at least a `template` pair if we got there
    for p in pairs.next().unwrap().into_inner() {
        match p.as_rule() {
            Rule::extends_tag => {
                let (ws, file) = parse_extends_include(p);
                nodes.push(Node::Extends(ws, file));
            }
            Rule::content => nodes.extend(parse_content(p)),
            Rule::comment_tag => (),
            _ => unreachable!("unknown tpl rule: {:?}", p.as_rule()),
        }
    }

    Ok(nodes)
}
