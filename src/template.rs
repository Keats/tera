use std::collections::HashMap;

use parser::{parse, Node};


// This is the parsed equivalent of a html template file
#[derive(Debug, Clone)]
pub struct Template {
    pub name: String, // filename
    pub ast: Node,
    pub parent: Option<String>,
    pub blocks: HashMap<String, Node>,
}

impl Template {
    pub fn new(tpl_name: &str, input: &str) -> Template {
        let ast = match parse(input) {
            Ok(a) => a,
            Err(e) => panic!("Error when parsing `{}`:\n{}", tpl_name, e)
        };

        // Figure out if there is a parent at compile time
        let parent = match ast.get_children().front() {
            Some(f) => match *f {
                Node::Extends(ref name) => Some(name.to_string()),
                _ => None
            },
            None => None
        };

        let mut blocks = HashMap::new();
        // If a template extends another, we only render the blocks node.
        // We find all those blocks at first so we don't need to do it for each render
        if parent.is_some() {
            for node in ast.get_children() {
                match node {
                    Node::Block { ref name, .. } => {
                        if blocks.contains_key(name) {
                            panic!("Error when parsing `{}`:\n{} block is duplicated", tpl_name, name);
                        }
                        blocks.insert(name.to_string(), node.clone())
                    },
                    _ => continue,
                };
            }
        }

        Template {
            name: tpl_name.to_string(),
            ast: ast,
            parent: parent,
            blocks: blocks,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::Template;

    #[test]
    fn test_can_parse_ok_template() {
        Template::new("hello", "Hello {{ world }}.");
    }

    #[test]
    fn test_can_find_parent_template() {
        let tpl = Template::new("hello", "{% extends \"base.html\" %}");

        assert_eq!(tpl.parent.unwrap(), "base.html".to_string());
    }

    #[test]
    fn test_can_find_blocks() {
        let tpl = Template::new(
            "hello",
            "{% extends \"base.html\" %}{% block hey %}{% endblock hey %}"
        );

        assert_eq!(tpl.parent.unwrap(), "base.html".to_string());
        assert_eq!(tpl.blocks.contains_key("hey"), true);
    }
}
