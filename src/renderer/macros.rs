use crate::errors::{Error, Result};
use crate::parser::ast::MacroDefinition;
use crate::template::Template;
use crate::tera::Tera;
use std::collections::HashMap;

// Types around Macros get complicated, simplify it a bit by using aliases

/// Maps { macro => macro_definition }
pub type MacroDefinitionMap = HashMap<String, MacroDefinition>;
/// Maps { namespace => ( macro_template, { macro => macro_definition }) }
pub type MacroNamespaceMap<'a> = HashMap<&'a str, (&'a str, &'a MacroDefinitionMap)>;
/// Maps { template => { namespace => ( macro_template, { macro => macro_definition }) }
pub type MacroTemplateMap<'a> = HashMap<&'a str, MacroNamespaceMap<'a>>;

/// Collection of all macro templates by file
#[derive(Clone, Debug, Default)]
pub struct MacroCollection<'a> {
    macros: MacroTemplateMap<'a>,
}

impl<'a> MacroCollection<'a> {
    pub fn from_original_template(tpl: &'a Template, tera: &'a Tera) -> MacroCollection<'a> {
        let mut macro_collection = MacroCollection { macros: MacroTemplateMap::new() };

        macro_collection
            .add_macros_from_template(tera, tpl)
            .expect("Couldn't load macros from base template");

        macro_collection
    }

    /// Add macros from parsed template to `MacroCollection`
    ///
    /// Macro templates can import other macro templates so the macro loading needs to
    /// happen recursively. We need all of the macros loaded in one go to be in the same
    /// HashMap for easy popping as well, otherwise there could be stray macro
    /// definitions remaining
    pub fn add_macros_from_template(
        &mut self,
        tera: &'a Tera,
        template: &'a Template,
    ) -> Result<()> {
        let template_name = &template.name[..];
        if self.macros.contains_key(template_name) {
            return Ok(());
        }

        let mut macro_namespace_map = MacroNamespaceMap::new();

        if !template.macros.is_empty() {
            macro_namespace_map.insert("self", (template_name, &template.macros));
        }

        for (filename, namespace) in &template.imported_macro_files {
            let macro_tpl = tera.get_template(filename)?;
            macro_namespace_map.insert(namespace, (filename, &macro_tpl.macros));
            self.add_macros_from_template(tera, macro_tpl)?;

            // We need to load the macros loaded in our macros in our namespace as well, unless we override it
            for (namespace, m) in &self.macros[&macro_tpl.name.as_ref()].clone() {
                if macro_namespace_map.contains_key(namespace) {
                    continue;
                }
                // We inserted before so we're safe
                macro_namespace_map.insert(namespace, *m);
            }
        }

        self.macros.insert(template_name, macro_namespace_map);

        for parent in &template.parents {
            let parent = &parent[..];
            let parent_template = tera.get_template(parent)?;
            self.add_macros_from_template(tera, parent_template)?;

            // We need to load the parent macros in our namespace as well, unless we override it
            for (namespace, m) in &self.macros[parent].clone() {
                if self.macros[template_name].contains_key(namespace) {
                    continue;
                }
                // We inserted before so we're safe
                self.macros.get_mut(template_name).unwrap().insert(namespace, *m);
            }
        }

        Ok(())
    }

    pub fn lookup_macro(
        &self,
        template_name: &'a str,
        macro_namespace: &'a str,
        macro_name: &'a str,
    ) -> Result<(&'a str, &'a MacroDefinition)> {
        let namespace = self
            .macros
            .get(template_name)
            .and_then(|namespace_map| namespace_map.get(macro_namespace));

        if let Some(n) = namespace {
            let &(macro_template, macro_definition_map) = n;

            if let Some(m) = macro_definition_map.get(macro_name).map(|md| (macro_template, md)) {
                Ok(m)
            } else {
                Err(Error::msg(format!(
                    "Macro `{}::{}` not found in template `{}`",
                    macro_namespace, macro_name, template_name
                )))
            }
        } else {
            Err(Error::msg(format!(
                "Macro namespace `{}` was not found in template `{}`. Have you maybe forgotten to import it, or misspelled it?",
                macro_namespace, template_name
            )))
        }
    }
}
