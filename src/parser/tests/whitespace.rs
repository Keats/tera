use parser::ast::*;
use parser::remove_whitespace;


#[test]
fn do_nothing_if_unneeded() {
    let ast = vec![Node::Text("hey ".to_string())];
    assert_eq!(remove_whitespace(ast.clone(), None), ast);
}

#[test]
fn remove_previous_ws_if_single_opening_tag_requires_it() {
    let ws = WS { left: true, right: false };
    let ast = vec![
        Node::Text("hey ".to_string()),
        Node::ImportMacro(ws, "hey ".to_string(), "ho".to_string()),
    ];

    assert_eq!(
        remove_whitespace(ast.clone(), None),
        vec![
            Node::Text("hey".to_string()), // it removed the trailing space
            Node::ImportMacro(ws, "hey ".to_string(), "ho".to_string()),
        ]
    );
}

#[test]
fn remove_next_ws_if_single_opening_tag_requires_it() {
    let ws = WS { left: true, right: true };
    let ast = vec![
        Node::ImportMacro(ws, "hey ".to_string(), "ho".to_string()),
        Node::Text("  hey".to_string()),
    ];

    assert_eq!(
        remove_whitespace(ast.clone(), None),
        vec![
            Node::ImportMacro(ws, "hey ".to_string(), "ho".to_string()),
            Node::Text("hey".to_string()), // it removed the leading space
        ]
    );
}

#[test]
fn handle_ws_both_sides_for_raw_tag() {
    let start_ws = WS { left: true, right: false };
    let end_ws = WS { left: true, right: true };
    let ast = vec![
        Node::Raw(start_ws, "  hey ".to_string(), end_ws),
        Node::Text("  hey".to_string()),
    ];

    assert_eq!(
        remove_whitespace(ast.clone(), None),
        vec![
            // it removed only the space at the end
            Node::Raw(start_ws, "  hey".to_string(), end_ws),
            Node::Text("hey".to_string()),
        ]
    );
}

#[test]
fn handle_ws_both_sides_for_forloop_tag_and_remove_empty_node() {
    let start_ws = WS { left: true, right: true };
    let end_ws = WS { left: true, right: true };
    let ast = vec![
        Node::Forloop(start_ws, Forloop {
            key: None,
            value: "item".to_string(),
            container: Expr::Int(1),
            // not valid but we don't care about it here
            body: vec![
                Node::Text("   ".to_string()),
                Node::Text("hey   ".to_string()),
            ],
        }, end_ws),
        Node::Text("  hey".to_string()),
    ];

    assert_eq!(
        remove_whitespace(ast.clone(), None),
        vec![
            Node::Forloop(start_ws, Forloop {
                key: None,
                value: "item".to_string(),
                container: Expr::Int(1),
                // not valid but we don't care about it here
                body: vec![
                    Node::Text("hey".to_string()),
                ],
            }, end_ws),
            Node::Text("hey".to_string()),
        ]
    );
}

#[test]
fn handle_ws_for_if_nodes() {
    let start_ws = WS { left: true, right: true };
    let end_ws = WS { left: false, right: true };
    let ast = vec![
        Node::Text("C ".to_string()),
        Node::If(If {
            conditions: vec![
                (WS { left: true, right: true }, Expr::Int(1), vec![Node::Text(" a ".to_string())]),
                (WS { left: true, right: false }, Expr::Int(1), vec![Node::Text(" a ".to_string())]),
                (WS { left: true, right: true }, Expr::Int(1), vec![Node::Text(" a ".to_string())]),
            ],
            otherwise: None,
        }, end_ws),
        Node::Text("  hey".to_string()),
    ];

    assert_eq!(
        remove_whitespace(ast.clone(), None),
        vec![
            Node::Text("C".to_string()),
            Node::If(If {
                conditions: vec![
                    (WS { left: true, right: true }, Expr::Int(1), vec![Node::Text("a".to_string())]),
                    (WS { left: true, right: false }, Expr::Int(1), vec![Node::Text(" a".to_string())]),
                    (WS { left: true, right: true }, Expr::Int(1), vec![Node::Text("a ".to_string())]),
                ],
                otherwise: None,
            }, end_ws),
            Node::Text("hey".to_string()),
        ]
    );
}

#[test]
fn handle_ws_for_if_nodes_with_else() {
    let start_ws = WS { left: true, right: true };
    let end_ws = WS { left: true, right: true };
    let ast = vec![
        Node::Text("C ".to_string()),
        Node::If(If {
            conditions: vec![
                (WS { left: true, right: true }, Expr::Int(1), vec![Node::Text(" a ".to_string())]),
                (WS { left: true, right: false }, Expr::Int(1), vec![Node::Text(" a ".to_string())]),
                (WS { left: true, right: true }, Expr::Int(1), vec![Node::Text(" a ".to_string())]),
            ],
            otherwise: Some((WS { left: true, right: true }, vec![Node::Text(" a ".to_string())])),
        }, end_ws),
        Node::Text("  hey".to_string()),
    ];

    assert_eq!(
        remove_whitespace(ast.clone(), None),
        vec![
            Node::Text("C".to_string()),
            Node::If(If {
                conditions: vec![
                    (WS { left: true, right: true }, Expr::Int(1), vec![Node::Text("a".to_string())]),
                    (WS { left: true, right: false }, Expr::Int(1), vec![Node::Text(" a".to_string())]),
                    (WS { left: true, right: true }, Expr::Int(1), vec![Node::Text("a".to_string())]),
                ],
                otherwise: Some((WS { left: true, right: true }, vec![Node::Text("a".to_string())])),
            }, end_ws),
            Node::Text("hey".to_string()),
        ]
    );
}
