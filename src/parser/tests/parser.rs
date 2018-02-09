use std::collections::HashMap;

use parser::parse;
use parser::ast::*;

#[test]
fn parse_empty_template() {
    let ast = parse("").unwrap();
    assert_eq!(ast.len(), 0);
}

#[test]
fn parse_text() {
    let ast = parse("hello world").unwrap();
    assert_eq!(ast[0], Node::Text("hello world".to_string()));
}

#[test]
fn parse_text_with_whitespace() {
    let ast = parse(" hello world ").unwrap();
    assert_eq!(ast[0], Node::Text(" hello world ".to_string()));
}

#[test]
fn parse_include_tag() {
    let ast = parse("{% include \"index.html\" -%}").unwrap();
    assert_eq!(
        ast[0],
        Node::Include(
            WS {
                left: false,
                right: true,
            },
            "index.html".to_string()
        )
    );
}

#[test]
fn parse_extends() {
    let ast = parse("{% extends \"index.html\" -%}").unwrap();
    assert_eq!(
        ast[0],
        Node::Extends(
            WS {
                left: false,
                right: true,
            },
            "index.html".to_string()
        )
    );
}

#[test]
fn parse_comments_before_extends() {
    let ast = parse("{# A comment #}{% extends \"index.html\" -%}").unwrap();
    assert_eq!(
        ast[0],
        Node::Extends(
            WS {
                left: false,
                right: true,
            },
            "index.html".to_string()
        )
    );
}

#[test]
fn parse_import_macro() {
    let ast = parse("{% import \"macros.html\" as macros -%}").unwrap();
    assert_eq!(
        ast[0],
        Node::ImportMacro(
            WS {
                left: false,
                right: true,
            },
            "macros.html".to_string(),
            "macros".to_string()
        )
    );
}

#[test]
fn parse_variable_tag_ident() {
    let ast = parse("{{ id }}").unwrap();
    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::new(ExprVal::Ident("id".to_string())))
    );
}

#[test]
fn parse_variable_tag_ident_with_simple_filters() {
    let ast = parse("{{ arr | first | join(n=2) }}").unwrap();
    let mut join_args = HashMap::new();
    join_args.insert("n".to_string(), Expr::new(ExprVal::Int(2)));

    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::with_filters(
            ExprVal::Ident("arr".to_string()),
            vec![
                FunctionCall {
                    name: "first".to_string(),
                    args: HashMap::new(),
                },
                FunctionCall {
                    name: "join".to_string(),
                    args: join_args,
                },
            ]
        ))
    );
}

#[test]
fn parse_variable_tag_lit() {
    let ast = parse("{{ 2 }}{{ 3.14 }}{{ \"hey\" }}{{ true }}").unwrap();
    assert_eq!(ast[0], Node::VariableBlock(Expr::new(ExprVal::Int(2))));
    assert_eq!(ast[1], Node::VariableBlock(Expr::new(ExprVal::Float(3.14))));
    assert_eq!(
        ast[2],
        Node::VariableBlock(Expr::new(ExprVal::String("hey".to_string())))
    );
    assert_eq!(ast[3], Node::VariableBlock(Expr::new(ExprVal::Bool(true))));
}

#[test]
fn parse_variable_tag_lit_math_expression() {
    let ast = parse("{{ count + 1 * 2.5 }}").unwrap();

    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::new(ExprVal::Math(MathExpr {
            lhs: Box::new(Expr::new(ExprVal::Ident("count".to_string()))),
            operator: MathOperator::Add,
            rhs: Box::new(Expr::new(ExprVal::Math(MathExpr {
                lhs: Box::new(Expr::new(ExprVal::Int(1))),
                operator: MathOperator::Mul,
                rhs: Box::new(Expr::new(ExprVal::Float(2.5))),
            }))),
        })))
    );
}

#[test]
fn parse_variable_tag_lit_math_expression_with_parentheses() {
    let ast = parse("{{ (count + 1) * 2.5 }}").unwrap();
    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::new(ExprVal::Math(MathExpr {
            lhs: Box::new(Expr::new(ExprVal::Math(MathExpr {
                lhs: Box::new(Expr::new(ExprVal::Ident("count".to_string()))),
                operator: MathOperator::Add,
                rhs: Box::new(Expr::new(ExprVal::Int(1))),
            }))),
            operator: MathOperator::Mul,
            rhs: Box::new(Expr::new(ExprVal::Float(2.5))),
        })))
    );
}

#[test]
fn parse_variable_tag_lit_math_expression_with_parentheses_and_filter() {
    let ast = parse("{{ (count + 1) * 2.5 | round }}").unwrap();
    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::with_filters(
            ExprVal::Math(MathExpr {
                lhs: Box::new(Expr::new(ExprVal::Math(MathExpr {
                    lhs: Box::new(Expr::new(ExprVal::Ident("count".to_string()))),
                    operator: MathOperator::Add,
                    rhs: Box::new(Expr::new(ExprVal::Int(1))),
                }))),
                operator: MathOperator::Mul,
                rhs: Box::new(Expr::new(ExprVal::Float(2.5))),
            }),
            vec![
                FunctionCall {
                    name: "round".to_string(),
                    args: HashMap::new(),
                },
            ],
        ))
    );
}

#[test]
fn parse_variable_math_on_filter() {
    let ast = parse("{{ a | length - 1 }}").unwrap();
    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::new(ExprVal::Math(MathExpr {
            lhs: Box::new(Expr::with_filters(
                ExprVal::Ident("a".to_string()),
                vec![
                    FunctionCall {
                        name: "length".to_string(),
                        args: HashMap::new(),
                    },
                ],
            )),
            operator: MathOperator::Sub,
            rhs: Box::new(Expr::new(ExprVal::Int(1))),
        })))
    );
}

#[test]
fn parse_variable_tag_simple_logic_expression() {
    let ast = parse("{{ 1 > 2 }}").unwrap();
    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::new(ExprVal::Logic(LogicExpr {
            lhs: Box::new(Expr::new(ExprVal::Int(1))),
            operator: LogicOperator::Gt,
            rhs: Box::new(Expr::new(ExprVal::Int(2))),
        })))
    );
}

#[test]
fn parse_variable_tag_math_and_logic_expression() {
    let ast = parse("{{ count + 1 * 2.5 and admin }}").unwrap();
    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::new(ExprVal::Logic(LogicExpr {
            lhs: Box::new(Expr::new(ExprVal::Math(MathExpr {
                lhs: Box::new(Expr::new(ExprVal::Ident("count".to_string()))),
                operator: MathOperator::Add,
                rhs: Box::new(Expr::new(ExprVal::Math(MathExpr {
                    lhs: Box::new(Expr::new(ExprVal::Int(1))),
                    operator: MathOperator::Mul,
                    rhs: Box::new(Expr::new(ExprVal::Float(2.5))),
                }))),
            }))),
            operator: LogicOperator::And,
            rhs: Box::new(Expr::new(ExprVal::Ident("admin".to_string()))),
        })))
    );
}

#[test]
fn parse_variable_tag_math_with_filters_and_logic_expression() {
    let ast = parse("{{ count + 1 * 2.5 | round and admin }}").unwrap();
    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::new(ExprVal::Logic(LogicExpr {
            lhs: Box::new(Expr::with_filters(
                ExprVal::Math(MathExpr {
                    lhs: Box::new(Expr::new(ExprVal::Ident("count".to_string()))),
                    operator: MathOperator::Add,
                    rhs: Box::new(Expr::new(ExprVal::Math(MathExpr {
                        lhs: Box::new(Expr::new(ExprVal::Int(1))),
                        operator: MathOperator::Mul,
                        rhs: Box::new(Expr::new(ExprVal::Float(2.5))),
                    }))),
                }),
                vec![
                    FunctionCall {
                        name: "round".to_string(),
                        args: HashMap::new(),
                    },
                ],
            )),
            operator: LogicOperator::And,
            rhs: Box::new(Expr::new(ExprVal::Ident("admin".to_string()))),
        })))
    );
}

#[test]
fn parse_variable_tag_simple_negated_expr() {
    let ast = parse("{{ not id }}").unwrap();
    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr {
            val: ExprVal::Ident("id".to_string()),
            filters: vec![],
            negated: true,
        })
    );
}

#[test]
fn parse_variable_tag_negated_expr() {
    let ast = parse("{{ not id and not true and not 1 + 1 }}").unwrap();
    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::new(ExprVal::Logic(LogicExpr {
            lhs: Box::new(Expr::new(ExprVal::Logic(LogicExpr {
                lhs: Box::new(Expr::new_negated(ExprVal::Ident("id".to_string()))),
                operator: LogicOperator::And,
                rhs: Box::new(Expr::new_negated(ExprVal::Bool(true))),
            }))),
            operator: LogicOperator::And,
            rhs: Box::new(Expr::new_negated(ExprVal::Math(MathExpr {
                lhs: Box::new(Expr::new(ExprVal::Int(1))),
                operator: MathOperator::Add,
                rhs: Box::new(Expr::new(ExprVal::Int(1))),
            }))),
        })))
    );
}

#[test]
fn parse_variable_tag_simple_test() {
    let ast = parse("{{ id is defined }}").unwrap();
    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::new(ExprVal::Test(Test {
            ident: "id".to_string(),
            name: "defined".to_string(),
            args: vec![],
        })))
    );
}

#[test]
fn parse_variable_tag_test_as_expression() {
    let ast = parse("{{ user is defined and user.admin }}").unwrap();
    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::new(ExprVal::Logic(LogicExpr {
            lhs: Box::new(Expr::new(ExprVal::Test(Test {
                ident: "user".to_string(),
                name: "defined".to_string(),
                args: vec![],
            }))),
            operator: LogicOperator::And,
            rhs: Box::new(Expr::new(ExprVal::Ident("user.admin".to_string()))),
        })))
    );
}

#[test]
fn parse_variable_tag_macro_call() {
    let ast = parse("{{ macros::get_time(some=1) }}").unwrap();
    let mut args = HashMap::new();
    args.insert("some".to_string(), Expr::new(ExprVal::Int(1)));

    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::new(ExprVal::MacroCall(MacroCall {
            namespace: "macros".to_string(),
            name: "get_time".to_string(),
            args,
        })))
    );
}

#[test]
fn parse_variable_tag_macro_call_with_filter() {
    let ast = parse("{{ macros::get_time(some=1) | round }}").unwrap();
    let mut args = HashMap::new();
    args.insert("some".to_string(), Expr::new(ExprVal::Int(1)));

    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::with_filters(
            ExprVal::MacroCall(MacroCall {
                namespace: "macros".to_string(),
                name: "get_time".to_string(),
                args,
            }),
            vec![
                FunctionCall {
                    name: "round".to_string(),
                    args: HashMap::new(),
                },
            ],
        ))
    );
}

#[test]
fn parse_variable_tag_global_function() {
    let ast = parse("{{ get_time(some=1) }}").unwrap();
    let mut args = HashMap::new();
    args.insert("some".to_string(), Expr::new(ExprVal::Int(1)));

    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::new(ExprVal::FunctionCall(FunctionCall {
            name: "get_time".to_string(),
            args,
        })))
    );
}

#[test]
fn parse_variable_tag_global_function_with_filter() {
    let ast = parse("{{ get_time(some=1) | round | upper }}").unwrap();
    let mut args = HashMap::new();
    args.insert("some".to_string(), Expr::new(ExprVal::Int(1)));

    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::with_filters(
            ExprVal::FunctionCall(FunctionCall {
                name: "get_time".to_string(),
                args,
            }),
            vec![
                FunctionCall {
                    name: "round".to_string(),
                    args: HashMap::new(),
                },
                FunctionCall {
                    name: "upper".to_string(),
                    args: HashMap::new(),
                },
            ],
        ))
    );
}

#[test]
fn parse_comment_tag() {
    let ast = parse("{# hey #}").unwrap();
    assert!(ast.is_empty());
}

#[test]
fn parse_set_tag_lit() {
    let ast = parse("{% set hello = \"hi\" %}").unwrap();
    assert_eq!(
        ast[0],
        Node::Set(
            WS::default(),
            Set {
                key: "hello".to_string(),
                value: Expr::new(ExprVal::String("hi".to_string())),
                global: false,
            }
        )
    );
}

#[test]
fn parse_set_tag_macro_call() {
    let ast = parse("{% set hello = macros::something() %}").unwrap();
    assert_eq!(
        ast[0],
        Node::Set(
            WS::default(),
            Set {
                key: "hello".to_string(),
                value: Expr::new(ExprVal::MacroCall(MacroCall {
                    namespace: "macros".to_string(),
                    name: "something".to_string(),
                    args: HashMap::new(),
                })),
                global: false,
            }
        )
    );
}

#[test]
fn parse_set_tag_fn_call() {
    let ast = parse("{% set hello = utcnow() %}").unwrap();
    assert_eq!(
        ast[0],
        Node::Set(
            WS::default(),
            Set {
                key: "hello".to_string(),
                value: Expr::new(ExprVal::FunctionCall(FunctionCall {
                    name: "utcnow".to_string(),
                    args: HashMap::new(),
                })),
                global: false,
            }
        )
    );
}

#[test]
fn parse_set_global_tag() {
    let ast = parse("{% set_global hello = utcnow() %}").unwrap();
    assert_eq!(
        ast[0],
        Node::Set(
            WS::default(),
            Set {
                key: "hello".to_string(),
                value: Expr::new(ExprVal::FunctionCall(FunctionCall {
                    name: "utcnow".to_string(),
                    args: HashMap::new(),
                })),
                global: true,
            }
        )
    );
}

#[test]
fn parse_raw_tag() {
    let ast = parse("{% raw -%}{{hey}}{%- endraw %}").unwrap();
    let mut start_ws = WS::default();
    start_ws.right = true;
    let mut end_ws = WS::default();
    end_ws.left = true;

    assert_eq!(ast[0], Node::Raw(start_ws, "{{hey}}".to_string(), end_ws));
}

#[test]
fn parse_filter_section_without_args() {
    let ast = parse("{% filter upper -%}A{%- endfilter %}").unwrap();
    let mut start_ws = WS::default();
    start_ws.right = true;
    let mut end_ws = WS::default();
    end_ws.left = true;

    assert_eq!(
        ast[0],
        Node::FilterSection(
            start_ws,
            FilterSection {
                filter: FunctionCall {
                    name: "upper".to_string(),
                    args: HashMap::new(),
                },
                body: vec![Node::Text("A".to_string())],
            },
            end_ws,
        )
    );
}

#[test]
fn parse_filter_section_with_args() {
    let ast = parse("{% filter upper(attr=1) -%}A{%- endfilter %}").unwrap();
    let mut start_ws = WS::default();
    start_ws.right = true;
    let mut end_ws = WS::default();
    end_ws.left = true;

    let mut args = HashMap::new();
    args.insert("attr".to_string(), Expr::new(ExprVal::Int(1)));

    assert_eq!(
        ast[0],
        Node::FilterSection(
            start_ws,
            FilterSection {
                filter: FunctionCall {
                    name: "upper".to_string(),
                    args,
                },
                body: vec![Node::Text("A".to_string())],
            },
            end_ws,
        )
    );
}

#[test]
fn parse_block() {
    let ast = parse("{% block hello %}{{super()}} hey{%- endblock hello %}").unwrap();
    let start_ws = WS::default();
    let mut end_ws = WS::default();
    end_ws.left = true;

    assert_eq!(
        ast[0],
        Node::Block(
            start_ws,
            Block {
                name: "hello".to_string(),
                body: vec![Node::Super, Node::Text(" hey".to_string())],
            },
            end_ws
        )
    );
}

#[test]
fn parse_simple_macro_definition() {
    let ast = parse("{% macro hello(a=1, b) %}A: {{a}}{% endmacro %}").unwrap();
    let mut args = HashMap::new();
    args.insert("a".to_string(), Some(Expr::new(ExprVal::Int(1))));
    args.insert("b".to_string(), None);

    assert_eq!(
        ast[0],
        Node::MacroDefinition(
            WS::default(),
            MacroDefinition {
                name: "hello".to_string(),
                args,
                body: vec![
                    Node::Text("A: ".to_string()),
                    Node::VariableBlock(Expr::new(ExprVal::Ident("a".to_string()))),
                ],
            },
            WS::default(),
        )
    );
}

#[test]
fn parse_value_forloop() {
    let ast = parse("{% for item in items | reverse %}A{%- endfor %}").unwrap();
    let start_ws = WS::default();
    let mut end_ws = WS::default();
    end_ws.left = true;

    assert_eq!(
        ast[0],
        Node::Forloop(
            start_ws,
            Forloop {
                key: None,
                value: "item".to_string(),
                container: Expr::with_filters(
                    ExprVal::Ident("items".to_string()),
                    vec![
                        FunctionCall {
                            name: "reverse".to_string(),
                            args: HashMap::new(),
                        },
                    ]
                ),
                body: vec![Node::Text("A".to_string())],
            },
            end_ws,
        )
    );
}

#[test]
fn parse_key_value_forloop() {
    let ast = parse("{% for key, item in get_map() %}A{%- endfor %}").unwrap();
    let start_ws = WS::default();
    let mut end_ws = WS::default();
    end_ws.left = true;

    assert_eq!(
        ast[0],
        Node::Forloop(
            start_ws,
            Forloop {
                key: Some("key".to_string()),
                value: "item".to_string(),
                container: Expr::new(ExprVal::FunctionCall(FunctionCall {
                    name: "get_map".to_string(),
                    args: HashMap::new(),
                })),
                body: vec![Node::Text("A".to_string())],
            },
            end_ws,
        )
    );
}

#[test]
fn parse_if() {
    let ast = parse("{% if item or admin %}A {%- elif 1 > 2 %}B{% else -%} C{%- endif %}").unwrap();
    let mut end_ws = WS::default();
    end_ws.left = true;

    let mut else_ws = WS::default();
    else_ws.right = true;

    assert_eq!(
        ast[0],
        Node::If(
            If {
                conditions: vec![
                    (
                        WS::default(),
                        Expr::new(ExprVal::Logic(LogicExpr {
                            lhs: Box::new(Expr::new(ExprVal::Ident("item".to_string()))),
                            operator: LogicOperator::Or,
                            rhs: Box::new(Expr::new(ExprVal::Ident("admin".to_string()))),
                        })),
                        vec![Node::Text("A ".to_string())],
                    ),
                    (
                        end_ws.clone(),
                        Expr::new(ExprVal::Logic(LogicExpr {
                            lhs: Box::new(Expr::new(ExprVal::Int(1))),
                            operator: LogicOperator::Gt,
                            rhs: Box::new(Expr::new(ExprVal::Int(2))),
                        })),
                        vec![Node::Text("B".to_string())],
                    ),
                ],
                otherwise: Some((else_ws, vec![Node::Text(" C".to_string())])),
            },
            end_ws,
        )
    );
}
