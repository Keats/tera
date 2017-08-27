use std::collections::HashMap;

use pest::Parser;
use pest::prec_climber::{PrecClimber, Operator, Assoc};
use pest::iterators::Pair;
use pest::inputs::Input;

use errors::{Result as TeraResult, ResultExt};

// This include forces recompiling this source file if the grammar file changes.
// Uncomment it when doing changes to the .pest file
// const _GRAMMAR: &'static str = include_str!("tera.pest");


#[derive(Parser)]
#[grammar = "parser/tera.pest"]
pub struct TeraParser;

pub mod ast;
#[cfg(test)]
mod tests;

use self::ast::*;

lazy_static! {
    static ref EXPR_CLIMBER: PrecClimber<Rule> = PrecClimber::new(vec![
        // <, <=, >, >=, ==, !=
        Operator::new(Rule::op_lt, Assoc::Left) | Operator::new(Rule::op_lte, Assoc::Left)
        | Operator::new(Rule::op_gt, Assoc::Left) | Operator::new(Rule::op_gte, Assoc::Left)
        | Operator::new(Rule::op_eq, Assoc::Left) | Operator::new(Rule::op_ineq, Assoc::Left),
        // +, -
        Operator::new(Rule::op_plus, Assoc::Left) | Operator::new(Rule::op_minus, Assoc::Left),
        // *, /
        Operator::new(Rule::op_times, Assoc::Left) | Operator::new(Rule::op_slash, Assoc::Left),
    ]);
    static ref LOGIC_EXPR_CLIMBER: PrecClimber<Rule> = PrecClimber::new(vec![
        Operator::new(Rule::op_or, Assoc::Left),
        Operator::new(Rule::op_and, Assoc::Left),
    ]);
}


fn parse_kwarg<I: Input>(pair: Pair<Rule, I>) -> (String, Expr) {
    let mut name = None;
    let mut val = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::ident => name = Some(p.into_span().as_str().to_string()),
            Rule::logic_expression => val = Some(parse_logic_expression(p)),
            _ => unreachable!("{:?} not supposed to get there (parse_kwarg)!", p.as_rule())
        };
    }

    (name.unwrap(), val.unwrap())
}

fn parse_fn_call<I: Input>(pair: Pair<Rule, I>) -> FunctionCall {
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

fn parse_filter<I: Input>(pair: Pair<Rule, I>) -> FunctionCall {
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

fn parse_ident<I: Input>(pair: Pair<Rule, I>) -> Ident {
    let mut name = None;
    let mut filters = vec![];

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::dotted_ident => name = Some(p.into_span().as_str().to_string()),
            Rule::filter => filters.push(parse_filter(p)),
            _ => unreachable!("{:?} not supposed to get there (parse_ident)!", p.as_rule())
        };
    }

    Ident { name: name.unwrap(), filters }
}

fn parse_test_call<I: Input>(pair: Pair<Rule, I>) -> (String, Vec<Expr>) {
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
                        args.push(parse_expression(p3));
                    }
                },
            _ => unreachable!("{:?} not supposed to get there (parse_test_call)!", p.as_rule())
        };
    }

    (name.unwrap(), args)
}

fn parse_test<I: Input>(pair: Pair<Rule, I>) -> Test {
    let mut ident = None;
    let mut name = None;
    let mut args = vec![];

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::context_ident => ident = Some(parse_ident(p)),
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

fn parse_expression<I: Input>(pair: Pair<Rule, I>) -> Expr {
    let primary = |pair| {
        parse_expression(pair)
    };

    let infix = |lhs: Expr, op: Pair<Rule, I>, rhs: Expr| {
        match op.as_rule() {
            Rule::op_plus | Rule::op_minus | Rule::op_times | Rule::op_slash => {
                Expr::Math(
                    MathExpr {
                        lhs: Box::new(lhs),
                        operator: match op.as_rule() {
                            Rule::op_plus => MathOperator::Add,
                            Rule::op_minus => MathOperator::Sub,
                            Rule::op_times => MathOperator::Mul,
                            Rule::op_slash => MathOperator::Div,
                            _ => unreachable!()
                        },
                        rhs: Box::new(rhs),
                    }
                )
            }
            Rule::op_lt | Rule::op_lte | Rule::op_gt | Rule::op_gte | Rule::op_ineq | Rule::op_eq => {
                Expr::Logic(
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
            }
            _ => unreachable!("{:?} not supposed to get there!", op.as_rule())
        }
    };

    match pair.as_rule() {
        Rule::int => Expr::Int(pair.into_span().as_str().parse().unwrap()),
        Rule::float => Expr::Float(pair.into_span().as_str().parse().unwrap()),
        Rule::boolean => {
            match pair.into_span().as_str() {
                "true" => Expr::Bool(true),
                "false" => Expr::Bool(false),
                _ => unreachable!(),
            }
        },
        Rule::test => Expr::Test(parse_test(pair)),
        Rule::string => Expr::String(pair.into_span().as_str().replace("\"", "").to_string()),
        Rule::context_ident => Expr::Ident(parse_ident(pair)),
        Rule::expression => EXPR_CLIMBER.climb(pair.into_inner(), primary, infix),
        _ => unreachable!("Got {:?} in parse_expression", pair.as_rule())
    }
}

/// An expression that can be negated
fn parse_logic_val<I: Input>(pair: Pair<Rule, I>) -> Expr {
    let mut negated = false;
    let mut expr = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::op_not => negated = true,
            Rule::expression => expr = Some(parse_expression(p)),
            _=> unreachable!(),
        };
    }

    if negated {
        Expr::Not(Box::new(expr.unwrap()))
    } else {
        expr.unwrap()
    }
}

fn parse_logic_expression<I: Input>(pair: Pair<Rule, I>) -> Expr {
    let primary = |pair: Pair<Rule, I>| {
        parse_logic_expression(pair)
    };

    let infix = |lhs: Expr, op: Pair<Rule, I>, rhs: Expr| {
        match op.as_rule() {
            Rule::op_or => {
                Expr::Logic(LogicExpr {
                    lhs: Box::new(lhs),
                    operator: LogicOperator::Or,
                    rhs: Box::new(rhs)
                })
            }
            Rule::op_and => {
                Expr::Logic(LogicExpr {
                    lhs: Box::new(lhs),
                    operator: LogicOperator::And,
                    rhs: Box::new(rhs)
                })
            }
            _ => unreachable!("{:?} not supposed to get there (infix of logic_expression)!", op.as_rule())
        }
    };

    // So this is a bit ugly but we want to get rid of the `op_not` rule when parsing
    // expression as this cannot output an expr on its own
    // TODO: redo that after thinking of a better way
    match pair.as_rule() {
        Rule::logic_val => parse_logic_val(pair),
        Rule::logic_expression => LOGIC_EXPR_CLIMBER.climb(pair.into_inner(), primary, infix),
        _ => unreachable!("Got {:?} in parse_logic_expression", pair.as_rule())
    }
}

fn parse_macro_call<I: Input>(pair: Pair<Rule, I>) -> Expr {
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

    Expr::MacroCall(MacroCall { namespace: namespace.unwrap(), name: name.unwrap(), args})
}

fn parse_variable_tag<I: Input>(pair: Pair<Rule, I>) -> Node {
    let p = pair.into_inner().nth(0).unwrap();
    match p.as_rule() {
        Rule::macro_call => Node::VariableBlock(parse_macro_call(p)),
        Rule::fn_call => Node::VariableBlock(Expr::FunctionCall(parse_fn_call(p))),
        Rule::logic_expression => Node::VariableBlock(parse_logic_expression(p)),
        _ => unreachable!()
    }
}

fn parse_import_macro<I: Input>(pair: Pair<Rule, I>) -> Node {
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
fn parse_extends_include<I: Input>(pair: Pair<Rule, I>) -> (WS, String) {
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

fn parse_set_tag<I: Input>(pair: Pair<Rule, I>) -> Node {
    let mut ws = WS::default();
    let mut key = None;
    let mut expr = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::tag_start => {
                ws.left = p.into_span().as_str() == "{%-";
            },
            Rule::tag_end => {
                ws.right = p.into_span().as_str() == "-%}";
            },
            Rule::ident => key = Some(p.as_str().to_string()),
            Rule::macro_call => panic!(),
            Rule::fn_call => panic!(),
            Rule::logic_expression => expr = Some(parse_logic_expression(p)),
            _ => unreachable!("unexpected {:?} rule in parse_set_tag", p.as_rule()),
        }
    }

    Node::Set(ws, Set {key: key.unwrap(), value: expr.unwrap()})
}

fn parse_raw_tag<I: Input>(pair: Pair<Rule, I>) -> Node {
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

fn parse_filter_section<I: Input>(pair: Pair<Rule, I>) -> Node {
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
                        _ => unreachable!(),
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

fn parse_block<I: Input>(pair: Pair<Rule, I>) -> Node {
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

fn parse_macro_definition<I: Input>(pair: Pair<Rule, I>) -> Node {
    let mut name = None;
    let mut args = HashMap::new();
    let mut body = vec![];

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::macro_tag => {
                for p2 in p.into_inner() {
                    match p2.as_rule() {
                        Rule::ident => name = Some(p2.as_str().to_string()),
                        Rule::macro_def_arg => {
                            let mut arg_name = None;
                            let mut default_val = None;
                            for p3 in p2.into_inner() {
                                match p3.as_rule() {
                                    Rule::ident => arg_name = Some(p3.as_str().to_string()),
                                    _ => default_val = Some(parse_expression(p3)),
                                };
                            }
                            args.insert(arg_name.unwrap(), default_val);
                        },
                        _ => continue,
                    };
                }
            },
            Rule::macro_content => body.extend(parse_content(p)),
            Rule::endmacro_tag => (),
            _ => unreachable!("unexpected {:?} rule in parse_macro_definition", p.as_rule()),
        }
    }

    Node::MacroDefinition(MacroDefinition {name: name.unwrap(), args, body})
}

fn parse_forloop<I: Input>(pair: Pair<Rule, I>) -> Node {
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
                        Rule::fn_call => container = Some(Expr::FunctionCall(parse_fn_call(p2))),
                        Rule::context_ident => container = Some(Expr::Ident(parse_ident(p2))),
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

fn parse_if<I: Input>(pair: Pair<Rule, I>) -> Node {
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
                        Rule::logic_expression => expr = Some(parse_logic_expression(p2)),
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

fn parse_content<I: Input>(pair: Pair<Rule, I>) -> Vec<Node> {
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
    // TODO: return a Result and rename the rules
    let mut pairs = TeraParser::parse_str(Rule::template, input)
        .unwrap_or_else(|e| panic!("{}", e));

    let mut nodes = vec![];

    // We must have at least a `template` pair if we got there
    for p in pairs.next().unwrap().into_inner() {
        match p.as_rule() {
            Rule::extends_tag => {
                let (ws, file) = parse_extends_include(p);
                nodes.push(Node::Extends(ws, file));
            }
            Rule::content => nodes.extend(parse_content(p)),
            _ => unreachable!("unknown tpl rule: {:?}", p.as_rule()),
        }
    }

    Ok(nodes)
}
