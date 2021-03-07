use crate::parser::ast::*;

macro_rules! trim_right_previous {
    ($vec: expr) => {
        if let Some(last) = $vec.pop() {
            if let Node::Text(mut s) = last {
                s = s.trim_end().to_string();
                if !s.is_empty() {
                    $vec.push(Node::Text(s));
                }
            } else {
                $vec.push(last);
            }
        }
    };
    ($cond: expr, $vec: expr) => {
        if $cond {
            trim_right_previous!($vec);
        }
    };
}

/// Removes whitespace from the AST nodes according to the `{%-` and `-%}` defined in the template.
/// Empty string nodes will be discarded.
///
/// The `ws` param is used when recursing through nested bodies to know whether to know
/// how to handle the whitespace for that whole body:
/// - set the initial `trim_left_next` to `ws.left`
/// - trim last node if it is a text node if `ws.right == true`
pub fn remove_whitespace(nodes: Vec<Node>, body_ws: Option<WS>) -> Vec<Node> {
    let mut res = Vec::with_capacity(nodes.len());

    // Whether the node we just added to res is a Text node
    let mut previous_was_text = false;
    // Whether the previous block ended wth `-%}` and we need to trim left the next text node
    let mut trim_left_next = body_ws.map_or(false, |ws| ws.left);

    for n in nodes {
        match n {
            Node::Text(s) => {
                previous_was_text = true;

                if !trim_left_next {
                    res.push(Node::Text(s));
                    continue;
                }
                trim_left_next = false;

                let new_val = s.trim_start();
                if !new_val.is_empty() {
                    res.push(Node::Text(new_val.to_string()));
                }
                // empty text nodes will be skipped
                continue;
            }
            Node::VariableBlock(ws, _)
            | Node::ImportMacro(ws, _, _)
            | Node::Extends(ws, _)
            | Node::Include(ws, _, _)
            | Node::Set(ws, _)
            | Node::Break(ws)
            | Node::Comment(ws, _)
            | Node::Continue(ws) => {
                trim_right_previous!(previous_was_text && ws.left, res);
                trim_left_next = ws.right;
            }
            Node::Raw(start_ws, ref s, end_ws) => {
                trim_right_previous!(previous_was_text && start_ws.left, res);
                previous_was_text = false;
                trim_left_next = end_ws.right;

                if start_ws.right || end_ws.left {
                    let val = if start_ws.right && end_ws.left {
                        s.trim()
                    } else if start_ws.right {
                        s.trim_start()
                    } else {
                        s.trim_end()
                    };

                    res.push(Node::Raw(start_ws, val.to_string(), end_ws));
                    continue;
                }
            }
            // Those nodes have a body surrounded by 2 tags
            Node::Forloop(start_ws, _, end_ws)
            | Node::MacroDefinition(start_ws, _, end_ws)
            | Node::FilterSection(start_ws, _, end_ws)
            | Node::Block(start_ws, _, end_ws) => {
                trim_right_previous!(previous_was_text && start_ws.left, res);
                previous_was_text = false;
                trim_left_next = end_ws.right;

                // let's remove ws from the bodies now and append the cleaned up node
                let body_ws = WS { left: start_ws.right, right: end_ws.left };
                match n {
                    Node::Forloop(_, mut forloop, _) => {
                        forloop.body = remove_whitespace(forloop.body, Some(body_ws));
                        res.push(Node::Forloop(start_ws, forloop, end_ws));
                    }
                    Node::MacroDefinition(_, mut macro_def, _) => {
                        macro_def.body = remove_whitespace(macro_def.body, Some(body_ws));
                        res.push(Node::MacroDefinition(start_ws, macro_def, end_ws));
                    }
                    Node::FilterSection(_, mut filter_section, _) => {
                        filter_section.body = remove_whitespace(filter_section.body, Some(body_ws));
                        res.push(Node::FilterSection(start_ws, filter_section, end_ws));
                    }
                    Node::Block(_, mut block, _) => {
                        block.body = remove_whitespace(block.body, Some(body_ws));
                        res.push(Node::Block(start_ws, block, end_ws));
                    }
                    _ => unreachable!(),
                };
                continue;
            }
            // The ugly one
            Node::If(If { conditions, otherwise }, end_ws) => {
                trim_left_next = end_ws.right;
                let mut new_conditions: Vec<(_, _, Vec<_>)> = Vec::with_capacity(conditions.len());

                for mut condition in conditions {
                    if condition.0.left {
                        // We need to trim the text node before the if tag
                        if new_conditions.is_empty() && previous_was_text {
                            trim_right_previous!(res);
                        } else if let Some(&mut (_, _, ref mut body)) = new_conditions.last_mut() {
                            trim_right_previous!(body);
                        }
                    }

                    // we can't peek at the next one to know whether we need to trim right since
                    // are consuming conditions. We'll find out at the next iteration.
                    condition.2 = remove_whitespace(
                        condition.2,
                        Some(WS { left: condition.0.right, right: false }),
                    );
                    new_conditions.push(condition);
                }

                previous_was_text = false;

                // We now need to look for the last potential `{%-` bit for if/elif

                // That can be a `{%- else`
                if let Some((else_ws, body)) = otherwise {
                    if else_ws.left {
                        if let Some(&mut (_, _, ref mut body)) = new_conditions.last_mut() {
                            trim_right_previous!(body);
                        }
                    }
                    let mut else_body =
                        remove_whitespace(body, Some(WS { left: else_ws.right, right: false }));
                    // if we have an `else`, the `endif` will affect the else node so we need to check
                    if end_ws.left {
                        trim_right_previous!(else_body);
                    }
                    res.push(Node::If(
                        If { conditions: new_conditions, otherwise: Some((else_ws, else_body)) },
                        end_ws,
                    ));
                    continue;
                }

                // Or `{%- endif`
                if end_ws.left {
                    if let Some(&mut (_, _, ref mut body)) = new_conditions.last_mut() {
                        trim_right_previous!(true, body);
                    }
                }

                res.push(Node::If(If { conditions: new_conditions, otherwise }, end_ws));
                continue;
            }
            Node::Super => (),
        };

        // If we are there, that means it's not a text node and we didn't have to modify the node
        previous_was_text = false;
        res.push(n);
    }

    if let Some(whitespace) = body_ws {
        trim_right_previous!(whitespace.right, res);
    }

    res
}
