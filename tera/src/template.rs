use crate::HashMap;
use crate::delimiters::Delimiters;
use crate::errors::{Error, ErrorKind, TeraResult};
use crate::parsing::ast::ComponentDefinition;
use crate::parsing::parser::Parser;
use crate::parsing::{Chunk, Compiler};
use crate::tera::Tera;
use crate::utils::Span;
use std::collections::HashSet;

#[derive(Debug, PartialEq, Clone)]
pub struct Template {
    pub name: String,
    pub(crate) source: String,
    pub(crate) path: Option<String>,
    pub(crate) chunk: Chunk,
    /// The blocks contained in this template only
    pub(crate) blocks: HashMap<String, Chunk>,
    /// Block definitions with their spans for error reporting
    pub(crate) block_name_spans: HashMap<String, Span>,
    pub(crate) components: HashMap<String, (ComponentDefinition, Chunk)>,
    pub(crate) component_calls: HashMap<String, Vec<Span>>,
    pub(crate) filter_calls: HashMap<String, Vec<Span>>,
    pub(crate) test_calls: HashMap<String, Vec<Span>>,
    pub(crate) function_calls: HashMap<String, Vec<Span>>,
    pub(crate) include_calls: HashMap<String, Vec<Span>>,
    /// The number of bytes of raw content in its parents and itself
    pub(crate) raw_content_num_bytes: usize,
    /// The full list of parent templates names
    pub(crate) parents: Vec<String>,
    pub(crate) block_lineage: HashMap<String, Vec<Chunk>>,
    /// Whether to auto-escape this template. It's set to `true` as default and will be updated
    /// when calling `Tera::autoescape_on` and when finalizing the templates
    pub(crate) autoescape_enabled: bool,
    /// The top level variables used by the template
    pub(crate) top_level_variables: HashSet<String>,
}

impl Template {
    pub(crate) fn new(
        tpl_name: &str,
        source: &str,
        path: Option<String>,
        delimiters: Delimiters,
    ) -> TeraResult<Self> {
        let parser = Parser::new(source, delimiters);
        let parser_output = match parser.parse() {
            Ok(p) => p,
            Err(e) => match e.kind {
                ErrorKind::SyntaxError(mut s) => {
                    s.set_source(tpl_name, source);
                    return Err(Error {
                        kind: ErrorKind::SyntaxError(s),
                        source: None,
                    });
                }
                _ => unreachable!("Parser got something other than a SyntaxError: {e}"),
            },
        };
        let parents = if let Some(p) = parser_output.parent {
            vec![p]
        } else {
            vec![]
        };

        let mut body_compiler = Compiler::new(tpl_name);
        body_compiler.compile(parser_output.nodes);

        // Optimize the main chunk
        let mut chunk = body_compiler.chunk;
        chunk.optimize();

        // Optimize all block chunks
        let blocks: HashMap<String, Chunk> = body_compiler
            .blocks
            .into_iter()
            .map(|(name, mut chunk)| {
                chunk.optimize();
                (name, chunk)
            })
            .collect();

        let raw_content_num_bytes = body_compiler.raw_content_num_bytes;

        let mut filter_calls = body_compiler.filter_calls;
        let mut test_calls = body_compiler.test_calls;
        let mut function_calls = body_compiler.function_calls;
        let mut include_calls = body_compiler.include_calls;
        let top_level_variables = body_compiler.top_level_variables;

        let components = parser_output
            .component_definitions
            .into_iter()
            .map(|c| {
                let mut compiler = Compiler::new(tpl_name);
                // We don't need the nodes again after it's compiled
                compiler.compile(c.body.clone());
                // Collect filter/test/function/include calls from component body
                for (name, spans) in compiler.filter_calls {
                    filter_calls.entry(name).or_default().extend(spans);
                }
                for (name, spans) in compiler.test_calls {
                    test_calls.entry(name).or_default().extend(spans);
                }
                for (name, spans) in compiler.function_calls {
                    function_calls.entry(name).or_default().extend(spans);
                }
                for (name, spans) in compiler.include_calls {
                    include_calls.entry(name).or_default().extend(spans);
                }
                let mut chunk = compiler.chunk;
                chunk.optimize();
                (c.name.clone(), (c, chunk))
            })
            .collect();
        let component_calls = body_compiler.component_calls;
        let block_name_spans = body_compiler.block_name_spans;

        Ok(Self {
            name: tpl_name.to_string(),
            source: source.to_string(),
            path,
            blocks,
            block_name_spans,
            raw_content_num_bytes,
            chunk,
            parents,
            components,
            component_calls,
            filter_calls,
            test_calls,
            function_calls,
            include_calls,
            top_level_variables,
            block_lineage: HashMap::new(),
            autoescape_enabled: true,
        })
    }

    pub(crate) fn size_hint(&self) -> usize {
        (self.raw_content_num_bytes * 2).next_power_of_two()
    }
}

/// Recursive fn that finds all the includes to detect if there are some cycles
pub(crate) fn check_include_cycles(tera: &Tera, start: &Template) -> Result<(), Error> {
    let mut stack: Vec<String> = vec![start.name.clone()];
    fn walk(tera: &Tera, current: &Template, stack: &mut Vec<String>) -> Result<(), Error> {
        let mut names: Vec<&String> = current.include_calls.keys().collect();
        names.sort();
        for include_name in names {
            let Some(resolved) = tera.resolve_template_name(include_name) else {
                continue;
            };
            if stack.iter().any(|s| s == resolved) {
                let mut chain = stack.clone();
                chain.push(resolved.to_string());
                return Err(Error::circular_include(resolved, chain));
            }
            stack.push(resolved.to_string());
            walk(tera, &tera.templates[resolved], stack)?;
            stack.pop();
        }
        Ok(())
    }
    walk(tera, start, &mut stack)
}

/// Recursive fn that finds all the parents and put them in an ordered Vec from closest to first parent
/// parent template
pub(crate) fn find_parents(
    tera: &Tera,
    start: &Template,
    template: &Template,
    mut parents: Vec<String>,
) -> Result<Vec<String>, Error> {
    if !parents.is_empty() && start.name == template.name {
        return Err(Error::circular_extend(&start.name, parents));
    }

    match template.parents.last() {
        Some(ref p) => match tera.resolve_template_name(p) {
            Some(resolved) => {
                let parent = &tera.templates[resolved];
                parents.push(parent.name.clone());
                find_parents(tera, start, parent, parents)
            }
            None => Err(Error::missing_parent(&template.name, p)),
        },
        None => {
            parents.reverse();
            Ok(parents)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_find_parents() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("a", ""),
            ("b", "{% extends 'a' %}"),
            ("c", "{% extends 'b' %}"),
        ])
        .unwrap();

        let parents_a =
            find_parents(&tera, &tera.templates["a"], &tera.templates["a"], vec![]).unwrap();
        assert!(parents_a.is_empty());

        let parents_b =
            find_parents(&tera, &tera.templates["b"], &tera.templates["b"], vec![]).unwrap();
        assert_eq!(parents_b, vec!["a".to_string()]);

        let parents_c =
            find_parents(&tera, &tera.templates["c"], &tera.templates["c"], vec![]).unwrap();
        assert_eq!(parents_c, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn nested_blocks_are_tracked() {
        let tpl = Template::new(
            "mid",
            r#"{% block hey %}hi {% block ending %}sincerely{% endblock ending %}{% endblock hey %}"#,
            None,
            Delimiters::default(),
        )
        .unwrap();
        // All blocks should be in the blocks map (for rendering)
        assert!(tpl.blocks.contains_key("hey"));
        assert!(tpl.blocks.contains_key("ending"));
        // Only top-level blocks should be in block_name_spans (for validation)
        // Nested blocks define new extension points, not overrides
        assert!(tpl.block_name_spans.contains_key("hey"));
        assert!(
            !tpl.block_name_spans.contains_key("ending"),
            "nested blocks should not be in block_name_spans"
        );
    }
}
