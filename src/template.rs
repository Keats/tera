use std::collections::HashMap;

use crate::errors::{Error, Result};
use crate::parser::ast::{Block, MacroDefinition, Node};
use crate::parser::{parse, remove_whitespace};

/// This is the parsed equivalent of a template file.
/// It also does some pre-processing to ensure it does as little as possible at runtime
/// Not meant to be used directly.
#[derive(Debug, Clone)]
pub struct Template {
    /// Name of the template, usually very similar to the path
    pub name: String,
    /// Original path of the file. A template doesn't necessarily have
    /// a file associated with it though so it's optional.
    pub path: Option<String>,
    /// Parsed AST, after whitespace removal
    pub ast: Vec<Node>,
    /// Whether this template came from a call to `Tera::extend`, so we do
    /// not remove it when we are doing a template reload
    pub from_extend: bool,

    /// Macros defined in that file: name -> definition ast
    pub macros: HashMap<String, MacroDefinition>,
    /// (filename, namespace) for the macros imported in that file
    pub imported_macro_files: Vec<(String, String)>,

    /// Only used during initial parsing. Rendering will use `self.parents`
    pub parent: Option<String>,
    /// Only used during initial parsing. Rendering will use `self.blocks_definitions`
    pub blocks: HashMap<String, Block>,

    // Below are filled when all templates have been parsed so we know the full hierarchy of templates
    /// The full list of parent templates
    pub parents: Vec<String>,
    /// The definition of all the blocks for the current template and the definition of those blocks
    /// in parent templates if there are some.
    /// Needed for super() to work without having to find them each time.
    /// The type corresponds to the following `block_name -> [(template name, definition)]`
    /// The order of the Vec is from the first in hierarchy to the current template and the template
    /// name is needed in order to load its macros if necessary.
    pub blocks_definitions: HashMap<String, Vec<(String, Block)>>,
}

impl Template {
    /// Parse the template string given
    pub fn new(tpl_name: &str, tpl_path: Option<String>, input: &str) -> Result<Template> {
        let ast = remove_whitespace(parse(input)?, None);

        // First we want all the blocks used in that template
        // This is recursive as we can have blocks inside blocks
        let mut blocks = HashMap::new();
        fn find_blocks(ast: &[Node], blocks: &mut HashMap<String, Block>) -> Result<()> {
            for node in ast {
                match *node {
                    Node::Block(_, ref block, _) => {
                        if blocks.contains_key(&block.name) {
                            return Err(Error::msg(format!(
                                "Block `{}` is duplicated",
                                block.name
                            )));
                        }

                        blocks.insert(block.name.to_string(), block.clone());
                        find_blocks(&block.body, blocks)?;
                    }
                    _ => continue,
                };
            }

            Ok(())
        }
        find_blocks(&ast, &mut blocks)?;

        // And now we find the potential parent and everything macro related (definition, import)
        let mut macros = HashMap::new();
        let mut imported_macro_files = vec![];
        let mut parent = None;

        for node in &ast {
            match *node {
                Node::Extends(_, ref name) => parent = Some(name.to_string()),
                Node::MacroDefinition(_, ref macro_def, _) => {
                    if macros.contains_key(&macro_def.name) {
                        return Err(Error::msg(format!(
                            "Macro `{}` is duplicated",
                            macro_def.name
                        )));
                    }
                    macros.insert(macro_def.name.clone(), macro_def.clone());
                }
                Node::ImportMacro(_, ref tpl_name, ref namespace) => {
                    imported_macro_files.push((tpl_name.to_string(), namespace.to_string()));
                }
                _ => continue,
            }
        }

        Ok(Template {
            name: tpl_name.to_string(),
            path: tpl_path,
            ast,
            parent,
            blocks,
            macros,
            imported_macro_files,
            parents: vec![],
            blocks_definitions: HashMap::new(),
            from_extend: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Template;

    #[test]
    fn can_parse_ok_template() {
        Template::new("hello", None, "Hello {{ world }}.").unwrap();
    }

    #[test]
    fn can_find_parent_template() {
        let tpl = Template::new("hello", None, "{% extends \"base.html\" %}").unwrap();

        assert_eq!(tpl.parent.unwrap(), "base.html".to_string());
    }

    #[test]
    fn can_find_blocks() {
        let tpl = Template::new(
            "hello",
            None,
            "{% extends \"base.html\" %}{% block hey %}{% endblock hey %}",
        )
        .unwrap();

        assert_eq!(tpl.parent.unwrap(), "base.html".to_string());
        assert!(tpl.blocks.contains_key("hey"));
    }

    #[test]
    fn can_find_nested_blocks() {
        let tpl = Template::new(
            "hello",
            None,
            "{% extends \"base.html\" %}{% block hey %}{% block extrahey %}{% endblock extrahey %}{% endblock hey %}",
        ).unwrap();

        assert_eq!(tpl.parent.unwrap(), "base.html".to_string());
        assert!(tpl.blocks.contains_key("hey"));
        assert!(tpl.blocks.contains_key("extrahey"));
    }

    #[test]
    fn can_find_macros() {
        let tpl = Template::new("hello", None, "{% macro hey() %}{% endmacro hey %}").unwrap();
        assert!(tpl.macros.contains_key("hey"));
    }

    #[test]
    fn can_find_imported_macros() {
        let tpl = Template::new("hello", None, "{% import \"macros.html\" as macros %}").unwrap();
        assert_eq!(
            tpl.imported_macro_files,
            vec![("macros.html".to_string(), "macros".to_string())]
        );
    }
}
