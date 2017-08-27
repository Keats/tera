use std::collections::HashMap;

use parser::parse;
use parser::ast::*;


#[test]
fn parse_empty_template() {
    let ast = parse("");
    assert_eq!(ast.len(), 0);
}

#[test]
fn parse_text() {
    let ast = parse("hello world");
    assert_eq!(ast[0], Node::Text("hello world".to_string()));
}

#[test]
fn parse_text_with_whitespace() {
    let ast = parse(" hello world ");
    assert_eq!(ast[0], Node::Text(" hello world ".to_string()));
}

#[test]
fn parse_include_tag() {
    let ast = parse("{% include \"index.html\" -%}");
    assert_eq!(ast[0], Node::Include(WS {left: false, right: true}, "index.html".to_string()));
}

#[test]
fn parse_extends() {
    let ast = parse("{% extends \"index.html\" -%}");
    assert_eq!(ast[0], Node::Extends(WS {left: false, right: true}, "index.html".to_string()));
}


#[test]
fn parse_import_macro() {
    let ast = parse("{% import \"macros.html\" as macros -%}");
    assert_eq!(
        ast[0],
        Node::ImportMacro(WS {left: false, right: true}, "macros.html".to_string(), "macros".to_string())
    );
}

#[test]
fn parse_variable_tag_ident() {
    let ast = parse("{{ id }}");
    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::Ident(Ident { name: "id".to_string(), filters: vec![] }))
    );
}

#[test]
fn parse_variable_tag_ident_with_simple_filters() {
    let ast = parse("{{ arr | first | join(n=2) }}");
    let mut join_args = HashMap::new();
    join_args.insert("n".to_string(), Expr::Int(2));

    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::Ident(Ident {
            name: "arr".to_string(),
            filters: vec![
                FunctionCall {name: "first".to_string(), args: HashMap::new() },
                FunctionCall {name: "join".to_string(), args: join_args },
            ],
        }))
    );
}

#[test]
fn parse_variable_tag_lit() {
    let ast = parse("{{ 2 }}{{ 3.14 }}{{ \"hey\" }}{{ true }}");
    assert_eq!(ast[0], Node::VariableBlock(Expr::Int(2)));
    assert_eq!(ast[1], Node::VariableBlock(Expr::Float(3.14)));
    assert_eq!(ast[2], Node::VariableBlock(Expr::String("hey".to_string())));
    assert_eq!(ast[3], Node::VariableBlock(Expr::Bool(true)));
}

#[test]
fn parse_variable_tag_lit_math_expression() {
    let ast = parse("{{ count + 1 * 2.5 }}");
    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::Math(MathExpr {
            lhs: Box::new(Expr::Ident(Ident {name: "count".to_string(), filters: vec![]})),
            operator: MathOperator::Add,
            rhs: Box::new(Expr::Math(MathExpr {
                lhs: Box::new(Expr::Int(1)),
                operator: MathOperator::Mul,
                rhs: Box::new(Expr::Float(2.5)),
            })),
        }))
    );
}

#[test]
fn parse_variable_tag_lit_math_expression_with_parentheses() {
    let ast = parse("{{ (count + 1) * 2.5 }}");
    assert_eq!(
        ast[0],
        Node::VariableBlock(
            Expr::Math(MathExpr {
                lhs: Box::new(Expr::Math(MathExpr {
                    lhs: Box::new(Expr::Ident(Ident { name: "count".to_string(), filters: vec![] })),
                    operator: MathOperator::Add,
                    rhs: Box::new(Expr::Int(1)),
                })),
                operator: MathOperator::Mul,
                rhs: Box::new(Expr::Float(2.5)),
            })
        )
    );
}

#[test]
fn parse_variable_tag_lit_logic_expression() {
    let ast = parse("{{ count + 1 * 2.5 and admin }}");
    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::Logic(LogicExpr {
            lhs: Box::new(Expr::Math(MathExpr {
                lhs: Box::new(Expr::Ident(Ident { name: "count".to_string(), filters: vec![] })),
                operator: MathOperator::Add,
                rhs: Box::new(Expr::Math(MathExpr {
                    lhs: Box::new(Expr::Int(1)),
                    operator: MathOperator::Mul,
                    rhs: Box::new(Expr::Float(2.5)),
                })),
            })),
            operator: LogicOperator::And,
            rhs: Box::new(Expr::Ident(Ident { name: "admin".to_string(), filters: vec![] })),
        }))
    );
}

#[test]
fn parse_variable_tag_simple_negated_expr() {
    let ast = parse("{{ not id }}");
    assert_eq!(
        ast[0],
        Node::VariableBlock(Expr::Not(Box::new(Expr::Ident(Ident { name: "id".to_string(), filters: vec![] }))))
    );
}

#[test]
fn parse_variable_tag_negated_expr() {
    let ast = parse("{{ not id and not true and not 1 + 1 }}");
    assert_eq!(
        ast[0],
        Node::VariableBlock(
            Expr::Logic(LogicExpr {
                lhs: Box::new(Expr::Logic(LogicExpr {
                    lhs: Box::new(Expr::Not(Box::new(Expr::Ident(Ident { name: "id".to_string(), filters: vec![] })))),
                    operator: LogicOperator::And,
                    rhs: Box::new(Expr::Not(Box::new(Expr::Bool(true)))),
                })),
                operator: LogicOperator::And,
                rhs: Box::new(Expr::Not(Box::new(Expr::Math(MathExpr {
                    lhs: Box::new(Expr::Int(1)),
                    operator: MathOperator::Add,
                    rhs: Box::new(Expr::Int(1)),
                })))),
            })
        )
    );
}

#[test]
fn parse_variable_tag_simple_test() {
    let ast = parse("{{ id is defined }}");
    assert_eq!(
        ast[0],
        Node::VariableBlock(
            Expr::Test(Test {
                ident: Ident {name: "id".to_string(), filters: vec![]},
                name: "defined".to_string(),
                args: vec![],
            })
        )
    );
}

#[test]
fn parse_variable_tag_simple_test_with_args() {
    let ast = parse("{{ id | squared is divisibleby(2) }}");
    assert_eq!(
        ast[0],
        Node::VariableBlock(
            Expr::Test(Test {
                ident: Ident {name: "id".to_string(), filters: vec![FunctionCall {name: "squared".to_string(), args: HashMap::new()}]},
                name: "divisibleby".to_string(),
                args: vec![Expr::Int(2)],
            })
        )
    );
}

#[test]
fn parse_variable_tag_test_as_expression() {
    let ast = parse("{{ user is defined and user.admin }}");
    assert_eq!(
        ast[0],
        Node::VariableBlock(
            Expr::Logic(LogicExpr {
                lhs: Box::new(Expr::Test(Test {
                    ident: Ident { name: "user".to_string(), filters: vec![] },
                    name: "defined".to_string(),
                    args: vec![],
                })),
                operator: LogicOperator::And,
                rhs: Box::new(Expr::Ident(Ident { name: "user.admin".to_string(), filters: vec![] })),
            })
        )
    );
}

#[test]
fn parse_variable_tag_macro_call_without_args() {
    let ast = parse("{{ macros::get_time() }}");

    assert_eq!(
        ast[0],
        Node::VariableBlock(
            Expr::MacroCall(MacroCall {
                namespace: "macros".to_string(),
                name: "get_time".to_string(),
                args: HashMap::new(),
            })
        )
    );
}

#[test]
fn parse_variable_tag_macro_call_with_args() {
    let ast = parse("{{ macros::get_time(some=1) }}");
    let mut args = HashMap::new();
    args.insert("some".to_string(), Expr::Int(1));

    assert_eq!(
        ast[0],
        Node::VariableBlock(
            Expr::MacroCall(MacroCall {
                namespace: "macros".to_string(),
                name: "get_time".to_string(),
                args,
            })
        )
    );
}

#[test]
fn parse_variable_tag_global_function() {
    let ast = parse("{{ get_time(some=1) }}");
    let mut args = HashMap::new();
    args.insert("some".to_string(), Expr::Int(1));

    assert_eq!(
        ast[0],
        Node::VariableBlock(
            Expr::FunctionCall(FunctionCall {
                name: "get_time".to_string(),
                args,
            })
        )
    );
}

#[test]
fn parse_comment_tag() {
    let ast = parse("{# hey #}");
    assert!(ast.is_empty());
}

#[test]
fn parse_set_tag_lit() {
    let ast = parse("{% set hello = \"hi\" %}");
    assert_eq!(
        ast[0],
        Node::Set(WS::default(), Set {
            key: "hello".to_string(),
            value: Expr::String("hi".to_string()),
        })
    );
}

#[test]
fn parse_raw_tag() {
    let ast = parse("{% raw -%}{{hey}}{%- endraw %}");
    let mut start_ws = WS::default();
    start_ws.right = true;
    let mut end_ws = WS::default();
    end_ws.left = true;

    assert_eq!(
        ast[0],
        Node::Raw(start_ws, "{{hey}}".to_string(),end_ws)
    );
}

#[test]
fn parse_filter_section() {
    let ast = parse("{% filter upper(attr=1) -%}A{%- endfilter %}");
    let mut start_ws = WS::default();
    start_ws.right = true;
    let mut end_ws = WS::default();
    end_ws.left = true;

    let mut args = HashMap::new();
    args.insert("attr".to_string(), Expr::Int(1));

    assert_eq!(
        ast[0],
        Node::FilterSection(
            start_ws,
            FilterSection {
                filter: FunctionCall {name: "upper".to_string(), args},
                body: vec![Node::Text("A".to_string())],
            },
            end_ws,
        )
    );
}

#[test]
fn parse_block() {
    let ast = parse("{% block hello %}{{super()}} hey{%- endblock hello %}");
    let start_ws = WS::default();
    let mut end_ws = WS::default();
    end_ws.left = true;

    assert_eq!(
        ast[0],
        Node::Block(
            start_ws,
            Block {name: "hello".to_string(), body: vec![Node::Super, Node::Text(" hey".to_string())]},
            end_ws
        )
    );
}

#[test]
fn parse_simple_macro_definition() {
    let ast = parse("{% macro hello(a=1, b) %}A: {{a}}{% endmacro %}");
    let mut args = HashMap::new();
    args.insert("a".to_string(), Some(Expr::Int(1)));
    args.insert("b".to_string(), None);

    assert_eq!(
        ast[0],
        Node::MacroDefinition(MacroDefinition {
            name: "hello".to_string(),
            args,
            body: vec![
                Node::Text("A: ".to_string()),
                Node::VariableBlock(Expr::Ident(Ident { name: "a".to_string(), filters: vec![] })),
            ],
        })
    );
}

#[test]
fn parse_value_forloop() {
    let ast = parse("{% for item in items | reverse %}A{%- endfor %}");
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
                container: Expr::Ident(Ident {
                    name: "items".to_string(),
                    filters: vec![FunctionCall {name: "reverse".to_string(), args: HashMap::new()}]
                }),
                body: vec![Node::Text("A".to_string())],
            },
            end_ws,
        )
    );
}

#[test]
fn parse_key_value_forloop() {
    let ast = parse("{% for key, item in get_map() %}A{%- endfor %}");
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
                container: Expr::FunctionCall(FunctionCall {
                    name: "get_map".to_string(),
                    args: HashMap::new(),
                }),
                body: vec![Node::Text("A".to_string())],
            },
            end_ws,
        )
    );
}
