use std::collections::{HashMap, LinkedList};

use parser::{parse, Node};
use errors::{Result};


/// This is the parsed equivalent of a template file
/// Not mean to be used directly unless you want a one-off template rendering
#[derive(Debug, Clone)]
pub struct Template {
    /// filename
    pub name: String,
    /// Parsed ast
    pub ast: Node,
    /// macros defined in that file
    pub macros: HashMap<String, Node>,
    /// (filename, namespace) for the macros imported in that file
    pub imported_macro_files: Vec<(String, String)>,
    /// Only used during initial parsing. Rendering will use `self.parents`
    pub parent: Option<String>,
    /// only used during initial parsing. Rendering will use `self.blocks_definitions`
    pub blocks: HashMap<String, Node>,

    /// Filled when all templates have been parsed: contains the full list of parent templates
    /// as opposed to Tera::Template which only contains the optional parent
    pub parents: Vec<String>,
    /// Filled when all templates have been parsed: contains the definition of all the blocks for
    /// the current template and the definition of parent templates if there is. Needed for super()
    /// to work without having to find them each time.
    /// The value is a Vec of all the definitions in order, with the tpl name from where it comes from
    /// Order is from highest in hierarchy to current template
    /// The tpl name is needed in order to load its macros
    pub blocks_definitions: HashMap<String, Vec<(String, Node)>>,
}

impl Template {
    /// Parse the template string given
    pub fn new(tpl_name: &str, input: &str) -> Result<Template> {
        let ast = parse(input)?;

        let mut blocks = HashMap::new();
        // We find all those blocks at first so we don't need to do it for each render
        // Recursive because we can have blocks inside blocks
        fn find_blocks(tpl_name: &str, ast: LinkedList<Node>, blocks: &mut HashMap<String, Node>) -> Result<()> {
            for node in ast {
                match node {
                    Node::Block { ref name, ref body } => {
                        if blocks.contains_key(name) {
                            bail!("Block `{}` is duplicated", name);
                        }
                        blocks.insert(name.to_string(), node.clone());
                        find_blocks(tpl_name, body.get_children(), blocks)?;
                    },
                    _ => continue,
                };
            }

            Ok(())
        }
        find_blocks(tpl_name, ast.get_children(), &mut blocks)?;

        // We also find all macros defined/imported in the template file
        let mut macros = HashMap::new();
        let mut imported_macro_files = vec![];
        let mut parent = None;
        for node in ast.get_children() {
            match node {
                Node::Extends(ref name) => {
                    parent = Some(name.to_string());
                },
                Node::Macro { ref name, .. } => {
                    if macros.contains_key(name) {
                        bail!("Macro `{}` is duplicated", name);
                    }
                    macros.insert(name.to_string(), node.clone());
                },
                Node::ImportMacro { tpl_name, name } => {
                    imported_macro_files.push((tpl_name.to_string(), name.to_string()));
                }
                _ => continue,
            };
        }

        Ok(Template {
            name: tpl_name.to_string(),
            ast: ast,
            parent: parent,
            blocks: blocks,
            macros: macros,
            imported_macro_files: imported_macro_files,

            parents: vec![],
            blocks_definitions: HashMap::new(),
        })
    }
}


#[cfg(test)]
mod tests {
    use super::Template;

    #[test]
    fn test_can_parse_ok_template() {
        Template::new("hello", "Hello {{ world }}.").unwrap();
    }

    #[test]
    fn test_can_find_parent_template() {
        let tpl = Template::new("hello", "{% extends \"base.html\" %}").unwrap();

        assert_eq!(tpl.parent.unwrap(), "base.html".to_string());
    }

    #[test]
    fn test_can_find_blocks() {
        let tpl = Template::new(
            "hello",
            "{% extends \"base.html\" %}{% block hey %}{% endblock hey %}"
        ).unwrap();

        assert_eq!(tpl.parent.unwrap(), "base.html".to_string());
        assert_eq!(tpl.blocks.contains_key("hey"), true);
    }

    #[test]
    fn test_can_find_nested_blocks() {
        let tpl = Template::new(
            "hello",
            "{% extends \"base.html\" %}{% block hey %}{% block extrahey %}{% endblock extrahey %}{% endblock hey %}"
        ).unwrap();

        assert_eq!(tpl.parent.unwrap(), "base.html".to_string());
        assert_eq!(tpl.blocks.contains_key("hey"), true);
        assert_eq!(tpl.blocks.contains_key("extrahey"), true);
    }

    #[test]
    fn test_can_find_macros() {
        let tpl = Template::new("hello", "{% macro hey() %}{% endmacro hey %}").unwrap();
        assert_eq!(tpl.macros.contains_key("hey"), true);
    }

    #[test]
    fn test_can_find_imported_macros() {
        let tpl = Template::new("hello", "{% import \"macros.html\" as macros %}").unwrap();
        assert_eq!(tpl.imported_macro_files, vec![("macros.html".to_string(), "macros".to_string())]);
    }
}
