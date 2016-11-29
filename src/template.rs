use std::collections::{HashMap, LinkedList};

use parser::{parse, Node};
use errors::TeraResult;
use errors::TeraError::MacroNotFound;


// This is the parsed equivalent of a template file
#[derive(Debug, Clone)]
pub struct Template {
    pub name: String, // filename
    pub ast: Node,
    pub macros: HashMap<String, Node>,
    // Only used during initial parsing. Rendering will use `self.parents`
    pub parent: Option<String>,
    // only used during initial parsing. Rendering will use `self.blocks_definitions`
    pub blocks: HashMap<String, Node>,
    // Filled when all templates have been parsed: contains the full list of parent templates
    // as opposed to Tera::Template which only contains the optional parent
    pub parents: Vec<String>,
    // Filled when all templates have been parsed: contains the definition of all the blocks for
    // the current template and the definition of parent templates if there is. Needed for super()
    // to work without having to find them each time
    pub blocks_definitions: HashMap<String, Vec<Node>>,
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

        // We find all those blocks at first so we don't need to do it for each render
        // Recursive because we can have blocks inside blocks
        fn find_blocks(tpl_name: String, ast: LinkedList<Node>,blocks: &mut HashMap<String, Node>) {
            //let t: () = blocks;
            for node in ast {
                match node {
                    Node::Block { ref name, ref body } => {
                        if blocks.contains_key(name) {
                            panic!("Error when parsing `{}`:\n{} block is duplicated", tpl_name, name);
                        }
                        blocks.insert(name.to_string(), node.clone());
                        find_blocks(tpl_name.clone(), body.get_children(), blocks);
                    },
                    _ => continue,
                };
            }
        }
        find_blocks(tpl_name.to_string(), ast.get_children(), &mut blocks);


        // We also find all macros defined in the template file
        let mut macros = HashMap::new();
        for node in ast.get_children() {
            match node {
                Node::Macro { ref name, .. } => {
                    if macros.contains_key(name) {
                        panic!("Error when parsing `{}`:\n{} macro is duplicated", tpl_name, name);
                    }
                    macros.insert(name.to_string(), node.clone())

                },
                _ => continue,
            };
        }

        Template {
            name: tpl_name.to_string(),
            ast: ast,
            parent: parent,
            blocks: blocks,
            macros: macros,

            parents: vec![],
            blocks_definitions: HashMap::new(),
        }
    }

    pub fn get_macro(&self, name: String) -> TeraResult<&Node> {
        match self.macros.get(&name) {
            Some(m) => Ok(m),
            None => Err(MacroNotFound(self.name.clone(), name.to_string())),
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

    #[test]
    fn test_can_find_nested_blocks() {
        let tpl = Template::new(
            "hello",
            "{% extends \"base.html\" %}{% block hey %}{% block extrahey %}{% endblock extrahey %}{% endblock hey %}"
        );

        assert_eq!(tpl.parent.unwrap(), "base.html".to_string());
        assert_eq!(tpl.blocks.contains_key("hey"), true);
        assert_eq!(tpl.blocks.contains_key("extrahey"), true);
    }

    #[test]
    fn test_can_find_macros() {
        let tpl = Template::new("hello", "{% macro hey() %}{% endmacro hey %}");
        assert_eq!(tpl.macros.contains_key("hey"), true);
    }
}
