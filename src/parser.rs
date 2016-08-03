use std::collections::LinkedList;

use pest::prelude::*;

use errors::{TeraResult, TeraError};


#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    List(LinkedList<Node>),

    Text(String),
    Int(i32),
    Float(f32),
    Bool(bool),

    Math {lhs: Box<Node>, rhs: Box<Node>, operator: String},
    Logic {lhs: Box<Node>, rhs: Box<Node>, operator: String},

    If {condition_nodes: LinkedList<Node>, else_node: Option<Box<Node>>},
    // represents a if/elif block and its body (body is a List)
    Conditional {condition: Box<Node>, body: Box<Node>},

    For {variable: String, array: String, body: Box<Node>},
    Block {name: String, body: Box<Node>},

    Identifier(String),
    Extends(String),
    VariableBlock(Box<Node>),
}

impl Node {
    pub fn get_children(&self) -> LinkedList<Node> {
        match *self {
            Node::List(ref l) => l.clone(),
            Node::If {ref condition_nodes, ..} => condition_nodes.clone(),
            _ => panic!("tried to get_children on a non-list/if node")
        }
    }
}

impl_rdp! {
    grammar! {
        whitespace = _{ !soi ~ ([" "] | ["\t"] | ["\r"] | ["\n"])+ ~ !eoi }

        // basic blocks of the language
        op_or        = { ["or"] }
        op_wrong_or  = { ["||"] }
        op_and       = { ["and"] }
        op_wrong_and = { ["&&"] }
        op_lte       = { ["<="] }
        op_gte       = { [">="] }
        op_lt        = { ["<"] }
        op_gt        = { [">"] }
        op_eq        = { ["=="] }
        op_ineq      = { ["!="] }
        op_plus      = { ["+"] }
        op_minus     = { ["-"] }
        op_times     = { ["*"] }
        op_slash     = { ["/"] }
        op_true      = { ["true"] }
        op_false     = { ["false"] }
        boolean      = _{ op_true | op_false }

        int   = @{ ["-"]? ~ (["0"] | ['1'..'9'] ~ ['0'..'9']*) }
        float = @{
            ["-"]? ~
                ["0"] ~ ["."] ~ ['0'..'9']+ |
                ['1'..'9'] ~ ['0'..'9']* ~ ["."] ~ ['0'..'9']+
        }

        identifier = @{
            (['a'..'z'] | ['A'..'Z'] | ["_"]) ~
            (['a'..'z'] | ['A'..'Z'] | ["_"] | ["."] | ['0'..'9'])*
        }
        // matches anything between 2 double quotes
        string = @{ ["\""] ~ (!(["\""]) ~ any )* ~ ["\""]}

        // Precedence climbing
        expression = _{
            // boolean first so they are not caught as identifiers
            { boolean | identifier | float | int }
            or          = { op_or | op_wrong_or }
            and         = { op_and | op_wrong_and }
            comparison  = { op_gt | op_lt | op_eq | op_ineq | op_lte | op_gte }
            add_sub     = { op_plus | op_minus }
            mul_div     = { op_times | op_slash }
        }

        // Tera specific things

        // different types of blocks
        variable_start = _{ ["{{"] }
        variable_end   = _{ ["}}"] }
        tag_start      = _{ ["{%"] }
        tag_end        = _{ ["%}"] }
        comment_start  = _{ ["{#"] }
        comment_end    = _{ ["#}"] }
        block_start    = _{ variable_start | tag_start | comment_start }

        // Actual tags
        extends_tag  = { tag_start ~ ["extends"] ~ string ~ tag_end }

        variable_tag    = { variable_start ~ expression ~ variable_end }
        comment_tag     = { comment_start ~ (!comment_end ~ any )* ~ comment_end }
        block_tag       = { tag_start ~ ["block"] ~ identifier ~ tag_end }
        if_tag          = { tag_start ~ ["if"] ~ expression ~ tag_end }
        elif_tag        = { tag_start ~ ["elif"] ~ expression ~ tag_end }
        else_tag        = { tag_start ~ ["else"] ~ tag_end }
        for_tag         = { tag_start ~ ["for"] ~ identifier ~ ["in"] ~ identifier ~ tag_end }
        endblock_tag    = { tag_start ~ ["endblock"] ~ identifier ~ tag_end }
        endif_tag       = { tag_start ~ ["endif"] ~ tag_end }
        endfor_tag      = { tag_start ~ ["endfor"] ~ tag_end }

        elif_block = { elif_tag ~ content* }

        text = @{ (!(block_start) ~ any )+ }
        content = {
            variable_tag |
            comment_tag |
            block_tag ~ content* ~ endblock_tag |
            if_tag ~ content* ~ elif_block* ~ (else_tag ~ content*)? ~ endif_tag |
            for_tag ~ content* ~ endfor_tag |
            text
        }

        // top level rule
        template = _{ soi ~ extends_tag? ~ content* ~ eoi }
    }

    process! {
        main(&self) -> TeraResult<Node> {
            (tpl: _template()) => {
                match tpl {
                    Ok(t) => Ok(Node::List(t)),
                    Err(e) => Err(e)
                }
            }
        }

        _template(&self) -> TeraResult<LinkedList<Node>> {
            (_: extends_tag, &name: string, tail: _template()) => {
                let mut tail2 = try!(tail);
                tail2.push_front(Node::Extends(name.replace("\"", "").to_string()));
                Ok(tail2)
            },
            (_: extends_tag, &name: string) => {
                let mut body = LinkedList::new();
                body.push_front(Node::Extends(name.replace("\"", "").to_string()));
                Ok(body)
            },
            (_: content, node: _content(), tail: _template()) => {
                let mut tail2 = try!(tail);
                match try!(node) {
                    Some(n) => { tail2.push_front(n); }
                    None => ()
                };
                Ok(tail2)
            },
            () => Ok(LinkedList::new())
        }

        // Option since we don't want comments in the AST
        _content(&self) -> TeraResult<Option<Node>> {
            (&head: text) => {
                Ok(Some(Node::Text(head.to_string())))
            },
            (_: variable_tag, exp: _expression()) => {
                Ok(Some(Node::VariableBlock(Box::new(try!(exp)))))
            },
            (_: block_tag, &name: identifier, body: _template(), _: endblock_tag, &end_name: identifier) => {
                if name != end_name {
                    let (line_no, col_no) = self.input().line_col(self.input.pos());
                    return Err(
                        TeraError::MismatchingEndBlock(
                            line_no, col_no, name.to_string(), end_name.to_string()
                        )
                    );
                }
                Ok(Some(Node::Block {
                    name: name.to_string(),
                    body: Box::new(Node::List(try!(body)))
                }))
            },
            (_: for_tag, &variable: identifier, &array: identifier, body: _template(), _: endfor_tag) => {
                Ok(Some(Node::For {
                    variable: variable.to_string(),
                    array: array.to_string(),
                    body: Box::new(Node::List(try!(body)))
                }))
            },
            // only if
            (_: if_tag, exp: _expression(), body: _template(), _: endif_tag) => {
                let mut condition_nodes = LinkedList::new();
                condition_nodes.push_front(Node::Conditional {
                    condition: Box::new(try!(exp)),
                    body: Box::new(Node::List(try!(body))),
                });

                Ok(Some(Node::If {
                    condition_nodes: condition_nodes,
                    else_node: None,
                }))
            },
            // if/elifs/else
            (_: if_tag, exp: _expression(), body: _template(), elifs: _elifs(), _: else_tag, else_body: _template(), _: endif_tag) => {
                let mut condition_nodes = LinkedList::new();
                condition_nodes.push_front(Node::Conditional {
                    condition: Box::new(try!(exp)),
                    body: Box::new(Node::List(try!(body))),
                });

                for elif in try!(elifs) {
                    condition_nodes.push_back(elif)
                }

                Ok(Some(Node::If {
                    condition_nodes: condition_nodes,
                    else_node: Some(Box::new(Node::List(try!(else_body)))),
                }))
            },
            // if/elifs
            (_: if_tag, exp: _expression(), body: _template(), elifs: _elifs(), _: endif_tag) => {
                let mut condition_nodes = LinkedList::new();
                condition_nodes.push_front(Node::Conditional {
                    condition: Box::new(try!(exp)),
                    body: Box::new(Node::List(try!(body))),
                });

                for elif in try!(elifs) {
                    condition_nodes.push_back(elif)
                }

                Ok(Some(Node::If {
                    condition_nodes: condition_nodes,
                    else_node: None,
                }))
            },
            // if/else
            (_: if_tag, exp: _expression(), body: _template(), _: else_tag, else_body: _template(), _: endif_tag) => {
                let mut condition_nodes = LinkedList::new();
                condition_nodes.push_front(Node::Conditional {
                    condition: Box::new(try!(exp)),
                    body: Box::new(Node::List(try!(body))),
                });

                Ok(Some(Node::If {
                    condition_nodes: condition_nodes,
                    else_node: Some(Box::new(Node::List(try!(else_body)))),
                }))
            },
            (_: comment_tag) => {
                Ok(None)
            }
        }

        _elifs(&self) -> TeraResult<LinkedList<Node>> {
            (_: elif_block, node: _if(), tail: _elifs()) => {
                let mut tail2 = try!(tail);
                tail2.push_front(try!(node));
                Ok(tail2)
            },
            () => Ok(LinkedList::new())
        }

        _if(&self) -> TeraResult<Node> {
            (_: if_tag, exp: _expression(), body: _template()) => {
                Ok(Node::Conditional {
                    condition: Box::new(try!(exp)),
                    body: Box::new(Node::List(try!(body))),
                })
            },
            (_: elif_tag, exp: _expression(), body: _template()) => {
                Ok(Node::Conditional {
                    condition: Box::new(try!(exp)),
                    body: Box::new(Node::List(try!(body))),
                })
            },
        }

        _expression(&self) -> TeraResult<Node> {
            (_: add_sub, left: _expression(), sign, right: _expression()) => {
                Ok(Node::Math {
                    lhs: Box::new(try!(left)),
                    rhs: Box::new(try!(right)),
                    operator: match sign.rule {
                        Rule::op_plus => "+".to_string(),
                        Rule::op_minus => "-".to_string(),
                        _ => unreachable!()
                    }
                })
            },
            (_: mul_div, left: _expression(), sign, right: _expression()) => {
                Ok(Node::Math {
                    lhs: Box::new(try!(left)),
                    rhs: Box::new(try!(right)),
                    operator: match sign.rule {
                        Rule::op_times => "*".to_string(),
                        Rule::op_slash => "/".to_string(),
                        _ => unreachable!()
                    }
                })
            },
            (_: comparison, left: _expression(), sign, right: _expression()) => {
                Ok(Node::Logic {
                    lhs: Box::new(try!(left)),
                    rhs: Box::new(try!(right)),
                    operator: match sign.rule {
                        Rule::op_gt => ">".to_string(),
                        Rule::op_lt => "<".to_string(),
                        Rule::op_eq => "==".to_string(),
                        Rule::op_ineq => "!=".to_string(),
                        Rule::op_lte => "<=".to_string(),
                        Rule::op_gte => ">=".to_string(),
                        _ => unreachable!()
                    }
                })
            },
            (_: and, left: _expression(), _, right: _expression()) => {
                Ok(Node::Logic {
                    lhs: Box::new(try!(left)),
                    rhs: Box::new(try!(right)),
                    operator: "and".to_string()
                })
            },
            (_: or, left: _expression(), _, right: _expression()) => {
                Ok(Node::Logic {
                    lhs: Box::new(try!(left)),
                    rhs: Box::new(try!(right)),
                    operator: "or".to_string()
                })
            },
            (&ident: identifier) => {
                Ok(Node::Identifier(ident.to_string()))
            },
            (&number: int) => {
                Ok(Node::Int(number.parse::<i32>().unwrap()))
            },
            (&number: float) => {
                Ok(Node::Float(number.parse::<f32>().unwrap()))
            },
            (_: op_true) => {
                Ok(Node::Bool(true))
            },
            (_: op_false) => {
                Ok(Node::Bool(false))
            },
            (&text: text) => {
                Ok(Node::Text(text.to_string()))
            },
        }
    }
}

// We need to preserve whitespace and count whitespace as text, which
// pest doesn't allow easily so we have a custom step before processing
// to add all/fix all our text tokens if necessary
pub fn parse(input: &str) -> TeraResult<Node> {
    let mut parser = Rdp::new(StringInput::new(input));

    if !parser.template() {
        let (_, pos) = parser.expected();
        let (line_no, col_no) = parser.input().line_col(pos);
        return Err(TeraError::InvalidSyntax(line_no, col_no));
    }

    // Tuples of (position_to_insert, token)
    let mut space_tokens = vec![];
    let mut previous_end = 0;
    // We need to check for 2 things:
    // 1. deprecated syntax -> error
    // 2. whitespace in between content to be replaced by text node later on
    for (i, token) in parser.queue().into_iter().enumerate() {
        match token.rule {
            // deprecated syntax first
            Rule::op_wrong_and => {
                let (line_no, col_no) = parser.input().line_col(token.start);
                return Err(
                    TeraError::DeprecatedSyntax(
                        line_no, col_no, "Use `and` instead of `&&`".to_string()
                    )
                );
            },
            Rule::op_wrong_or => {
                let (line_no, col_no) = parser.input().line_col(token.start);
                return Err(
                    TeraError::DeprecatedSyntax(
                        line_no, col_no, "Use `or` instead of `||`".to_string()
                    )
                );
            },
            // All possible tags showing up in the content rule to handle whitespace
            Rule::variable_tag | Rule::comment_tag | Rule::if_tag | Rule::else_tag | Rule::text
            | Rule::endif_tag | Rule::endblock_tag | Rule::endfor_tag
            | Rule::elif_tag | Rule::block_tag | Rule::for_tag | Rule::extends_tag => {
                if previous_end > 0 {
                    // We need to take into account the Rule::content so we insert
                    // before it if there is one (endblock tag don't have one typically)
                    let insert_at = match token.rule {
                        Rule::endif_tag | Rule::endblock_tag | Rule::endfor_tag => i,
                        _ => i - 1
                    };
                    if previous_end < token.start {
                        space_tokens.push((
                            insert_at,
                            Token::new(Rule::text, previous_end, token.start)
                        ));
                    }
                }
                previous_end = token.end;
            },
            _ => ()
        };
    }

    // println!("{:?}", parser.queue());
    // println!("{:?}", space_tokens);
    // Next we need to insert the space tokens
    let mut number_inserted = 0;
    for (i, token) in space_tokens {
        parser.queue_mut().insert(i + number_inserted, token);
        // process! expect text to be wrapped in a Token::content
        parser.queue_mut().insert(i + number_inserted, Token {
            rule: Rule::content,
            start: token.start,
            end: token.end
        });
        number_inserted += 2;
    }
    // println!("{:?}", input);
    // println!("{:?}", parser.queue());
    parser.main()
}

#[cfg(test)]
mod tests {
    use std::collections::LinkedList;

    use pest::prelude::*;
    use super::{Rdp, Node, parse};
    use errors::TeraError;

    #[test]
    fn test_int() {
        let mut parser = Rdp::new(StringInput::new("123"));
        assert!(parser.int());
        assert!(parser.end());
    }

    #[test]
    fn test_float() {
        let mut parser = Rdp::new(StringInput::new("123.5"));
        assert!(parser.float());
        assert!(parser.end());
    }

    #[test]
    fn test_identifier() {
        let mut parser = Rdp::new(StringInput::new("client.phone_number"));
        assert!(parser.identifier());
        assert!(parser.end());
    }

    #[test]
    fn test_text() {
        let mut parser = Rdp::new(StringInput::new("Hello\n 世界"));
        assert!(parser.text());
        assert!(parser.end());
    }

    #[test]
    fn test_text_with_trailing_space() {
        let mut parser = Rdp::new(StringInput::new("Hello\n 世界  "));
        assert!(parser.text());
        assert!(parser.end());
    }

    #[test]
    fn test_text_with_leading_space() {
        let mut parser = Rdp::new(StringInput::new("   Hello\n 世界"));
        assert!(parser.text());
        assert!(parser.end());
    }

    #[test]
    fn test_string() {
        let mut parser = Rdp::new(StringInput::new("\"Blabla\""));
        assert!(parser.string());
        assert!(parser.end());
    }

    #[test]
    fn test_extends_tag() {
        let mut parser = Rdp::new(StringInput::new("{% extends \"base.html\" %}"));
        assert!(parser.extends_tag());
        assert!(parser.end());
    }

    #[test]
    fn test_comment_tag() {
        let mut parser = Rdp::new(StringInput::new("{# some text {{}} #}"));
        assert!(parser.comment_tag());
        assert!(parser.end());
    }

    #[test]
    fn test_block_tag() {
        let mut parser = Rdp::new(StringInput::new("{% block hello %}"));
        assert!(parser.block_tag());
        assert!(parser.end());
    }

    #[test]
    fn test_endblock_tag() {
        let mut parser = Rdp::new(StringInput::new("{% endblock hello %}"));
        assert!(parser.endblock_tag());
        assert!(parser.end());
    }

    #[test]
    fn test_for_tag() {
        let mut parser = Rdp::new(StringInput::new("{% for client in clients %}"));
        assert!(parser.for_tag());
        assert!(parser.end());
    }

    #[test]
    fn test_endfor_tag() {
        let mut parser = Rdp::new(StringInput::new("{% endfor %}"));
        assert!(parser.endfor_tag());
        assert!(parser.end());
    }

    #[test]
    fn test_expression_math() {
        let mut parser = Rdp::new(StringInput::new("1 + 2 + 3 * 9/2 + 2"));
        assert!(parser.expression());
        assert!(parser.end());
    }

    #[test]
    fn test_expression_identifier_logic_simple() {
        let mut parser = Rdp::new(StringInput::new("index + 1 > 1"));
        assert!(parser.expression());
        assert!(parser.end());
    }

    #[test]
    fn test_expression_identifier_logic_complex() {
        let mut parser = Rdp::new(StringInput::new("1 > 2 or 3 == 4 and admin"));
        assert!(parser.expression());
        assert!(parser.end());
    }

    #[test]
    fn test_if_tag() {
        let mut parser = Rdp::new(StringInput::new("{% if true or show == false %}"));
        assert!(parser.if_tag());
        assert!(parser.end());
    }

    #[test]
    fn test_variable_tag() {
        let mut parser = Rdp::new(StringInput::new("{{loop.index + 1}}"));
        assert!(parser.variable_tag());
        assert!(parser.end());
    }

    #[test]
    fn test_content() {
        let mut parser = Rdp::new(StringInput::new("{% if i18n %}世界{% else %}world{% endif %}"));
        assert!(parser.content());
        assert!(parser.end());
    }

    #[test]
    fn test_template() {
        let mut parser = Rdp::new(StringInput::new("
            {# Greeter template #}
            Hello {% if i18n %}世界{% else %}world{% endif %}
            {% for country in countries %}
                {{ loop.index }}.{{ country }}
            {% endfor %}
        "));
        assert!(parser.template());
        assert!(parser.end());
    }

    #[test]
    fn test_invalid_syntax() {
        let parsed_ast = parse("{% block hey ");
        assert!(parsed_ast.is_err());
        assert_eq!(
            parsed_ast.err().unwrap(),
            TeraError::InvalidSyntax(1, 13)
        );
    }

    #[test]
    fn test_invalid_extends() {
        let parsed_ast = parse("{% extends \"base.html\" %} {% extends \"base.html\" %}");
        assert!(parsed_ast.is_err());
        assert_eq!(
            parsed_ast.err().unwrap(),
            TeraError::InvalidSyntax(1, 30)
        );
    }

    #[test]
    fn test_ast_basic() {
        let parsed_ast = parse(" Hello {{ count + 1 * 2.5 }} {{ true or false and 1 }}");
        let mut ast = LinkedList::new();
        ast.push_front(Node::VariableBlock(
            Box::new(Node::Logic {
                lhs: Box::new(Node::Bool(true)),
                rhs: Box::new(Node::Logic {
                    lhs: Box::new(Node::Bool(false)),
                    rhs: Box::new(Node::Int(1)),
                    operator: "and".to_string()
                }),
                operator: "or".to_string()
            })
        ));
        ast.push_front(Node::Text(" ".to_string()));
        ast.push_front(Node::VariableBlock(
            Box::new(Node::Math {
                lhs: Box::new(Node::Identifier("count".to_string())),
                rhs: Box::new(Node::Math {
                    lhs: Box::new(Node::Int(1)),
                    rhs: Box::new(Node::Float(2.5)),
                    operator: "*".to_string()
                }),
                operator: "+".to_string()
            })
        ));
        ast.push_front(Node::Text(" Hello ".to_string()));

        let root = Node::List(ast);
        assert_eq!(parsed_ast.unwrap(), root);
    }

    #[test]
    fn test_ast_block() {
        let parsed_ast = parse("{% block content %}Hello{% endblock content %}");
        let mut ast = LinkedList::new();
        let mut inner_content = LinkedList::new();
        inner_content.push_front(Node::Text("Hello".to_string()));
        ast.push_front(Node::Block {
            name: "content".to_string(),
            body: Box::new(Node::List(inner_content))
        });
        let root = Node::List(ast);
        assert_eq!(parsed_ast.unwrap(), root);
    }

    #[test]
    fn test_ast_for() {
        let parsed_ast = parse("{% for user in users %}{{user.email}}{% endfor %}");
        let mut ast = LinkedList::new();
        let mut inner_content = LinkedList::new();
        inner_content.push_front(Node::VariableBlock(
            Box::new(Node::Identifier("user.email".to_string()))
        ));
        ast.push_front(Node::For {
            variable: "user".to_string(),
            array: "users".to_string(),
            body: Box::new(Node::List(inner_content))
        });
        let root = Node::List(ast);
        assert_eq!(parsed_ast.unwrap(), root);
    }

    #[test]
    fn test_ast_extends() {
        let parsed_ast = parse("{% extends \"base.html\" %}");
        let mut ast = LinkedList::new();
        ast.push_front(Node::Extends("base.html".to_string()));
        let root = Node::List(ast);
        assert_eq!(parsed_ast.unwrap(), root);
    }

    #[test]
    fn test_ast_if() {
        let parsed_ast = parse("{% if superadmin %}Hey{% endif %}");
        let mut ast = LinkedList::new();
        let mut body = LinkedList::new();
        body.push_front(Node::Text("Hey".to_string()));

        let mut condition_nodes = LinkedList::new();
        condition_nodes.push_front(Node::Conditional {
            condition: Box::new(Node::Identifier("superadmin".to_string())),
            body: Box::new(Node::List(body.clone()))
        });

        ast.push_front(Node::If {
            condition_nodes: condition_nodes,
            else_node: None,
        });
        let root = Node::List(ast);
        assert_eq!(parsed_ast.unwrap(), root);
    }

    #[test]
    fn test_ast_if_else() {
        let parsed_ast = parse("{% if superadmin %}Hey{% else %}Hey{% endif %}");
        let mut ast = LinkedList::new();
        let mut body = LinkedList::new();
        body.push_front(Node::Text("Hey".to_string()));

        let mut condition_nodes = LinkedList::new();
        condition_nodes.push_front(Node::Conditional {
            condition: Box::new(Node::Identifier("superadmin".to_string())),
            body: Box::new(Node::List(body.clone()))
        });

        ast.push_front(Node::If {
            condition_nodes: condition_nodes,
            else_node: Some(Box::new(Node::List(body.clone()))),
        });
        let root = Node::List(ast);
        assert_eq!(parsed_ast.unwrap(), root);
    }

    #[test]
    fn test_ast_if_elif() {
        let parsed_ast = parse("{% if superadmin %}Hey{% elif admin %}Hey{% endif %}");
        let mut ast = LinkedList::new();
        let mut body = LinkedList::new();
        body.push_front(Node::Text("Hey".to_string()));

        let mut condition_nodes = LinkedList::new();
        condition_nodes.push_front(Node::Conditional {
            condition: Box::new(Node::Identifier("superadmin".to_string())),
            body: Box::new(Node::List(body.clone()))
        });
        condition_nodes.push_back(Node::Conditional {
            condition: Box::new(Node::Identifier("admin".to_string())),
            body: Box::new(Node::List(body.clone()))
        });

        ast.push_front(Node::If {
            condition_nodes: condition_nodes,
            else_node: None,
        });
        let root = Node::List(ast);
        assert_eq!(parsed_ast.unwrap(), root);
    }

    #[test]
    fn test_ast_if_elifs_else() {
        let parsed_ast = parse("{% if superadmin %}Hey{% elif admin %}Hey{% else %}Hey{% endif %}");
        let mut ast = LinkedList::new();
        let mut body = LinkedList::new();
        body.push_front(Node::Text("Hey".to_string()));

        let mut condition_nodes = LinkedList::new();
        condition_nodes.push_back(Node::Conditional {
            condition: Box::new(Node::Identifier("admin".to_string())),
            body: Box::new(Node::List(body.clone()))
        });
        condition_nodes.push_front(Node::Conditional {
            condition: Box::new(Node::Identifier("superadmin".to_string())),
            body: Box::new(Node::List(body.clone()))
        });

        ast.push_front(Node::If {
            condition_nodes: condition_nodes,
            else_node: Some(Box::new(Node::List(body.clone()))),
        });
        let root = Node::List(ast);
        assert_eq!(parsed_ast.unwrap(), root);
    }

    #[test]
    fn test_ast_error_old_and() {
        let parsed_ast = parse("{{ true && 1 }}");
        assert!(parsed_ast.is_err());
        assert_eq!(
            parsed_ast.err().unwrap(),
            TeraError::DeprecatedSyntax(1, 9, "Use `and` instead of `&&`".to_string())
        );
    }

    #[test]
    fn test_ast_error_old_or() {
        let parsed_ast = parse("{{ true || 1 }}");
        assert!(parsed_ast.is_err());
        assert_eq!(
            parsed_ast.err().unwrap(),
            TeraError::DeprecatedSyntax(1, 9, "Use `or` instead of `||`".to_string())
        );
    }

    #[test]
    fn test_ast_error_mismatch_endblock_name() {
        let parsed_ast = parse("{% block hey %}{% endblock ho %}");
        assert!(parsed_ast.is_err());
        assert_eq!(
            parsed_ast.err().unwrap(),
            TeraError::MismatchingEndBlock(1, 33, "hey".to_string(), "ho".to_string())
        );
    }

    // Test that we can parse the template used in benching
    #[test]
    fn test_parse_bench() {
        let parsed_ast = parse("
            <html>
              <head>
                <title>{{ product.name }}</title>
              </head>
              <body>
                <h1>{{ product.name }} - {{ product.manufacturer }}</h1>
                <p>{{ product.summary }}</p>
                <p>£{{ product.price * 1.20 }} (VAT inc.)</p>
                <p>Look at reviews from your friends {{ username }}</p>
                <button>Buy!</button>
              </body>
            </html>
        ");
        assert!(parsed_ast.is_ok());
    }
}
