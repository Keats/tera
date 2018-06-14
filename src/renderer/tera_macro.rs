//! For processing macro invocations

// --- module use statements ---

use errors::{Result, ResultExt};
use parser::ast::MacroDefinition;
use std::collections::HashMap;
use template::Template;
use tera::Tera;

// --- module type aliases ---

/// Maps { macro => macro_definition }
type MacroDefinitionMap = HashMap<String, MacroDefinition>;

/// Maps { namespace => ( macro_template, { macro => macro_definition })}
type MacroNamespaceMap<'a> = HashMap<&'a str, (&'a str, &'a MacroDefinitionMap)>;

/// Maps { template => { namespace { macro => macro_definition }}}
type MacroTemplateMap<'a> = HashMap<&'a str, MacroNamespaceMap<'a>>;

// --- module struct definitions ---

/// Collection of all macro templates by file
#[derive(Clone, Debug, Default)]
pub struct MacroCollection<'a> {
  macro_template_map: MacroTemplateMap<'a>,
}
/// Implementation for type `MacroCollection`.
impl<'a> MacroCollection<'a> {
  /// Read macros from template file recursively
  ///
  ///  * `template_name` - Name of macro template file
  ///  * `tera` - Houses other templates, filters, global functions, etc
  ///  * _return_ - Macro definitions from a template file
  ///
  pub fn from_template_original(
    template_name: &'a str,
    tera: &'a Tera,
  ) -> Result<MacroCollection<'a>> {
    let mut macro_collection = MacroCollection {
      macro_template_map: MacroTemplateMap::new(),
    };

    macro_collection.add_macros_from_template(tera, tera.get_template(template_name)?)?;

    Ok(macro_collection)
  }

  /// Add macros from parsed template to `MacroCollection`
  ///
  /// Macro templates can import other macro templates so the macro loading needs to
  /// happen recursively. We need all of the macros loaded in one go to be in the same
  /// hashmap for easy popping as well, otherwise there could be stray macro
  /// definitions remaining
  ///
  ///  * `tera` - Tera housing templates
  ///  * `template` - Template to add macros from
  ///  * _return_ - Errors if template not found
  ///
  pub fn add_macros_from_template(
    self: &mut Self,
    tera: &'a Tera,
    template: &'a Template,
  ) -> Result<()> {
    let template_name = &template.name[..];
    if self.macro_template_map.contains_key(template_name) {
      return Ok(());
    }

    let mut macro_namespace_map = MacroNamespaceMap::new();

    if !template.macros.is_empty() {
      macro_namespace_map.insert("self", (template_name, &template.macros));
    }

    for &(ref filename, ref namespace) in &template.imported_macro_files {
      let macro_tpl = tera.get_template(filename)?;
      macro_namespace_map.insert(namespace, (filename, &macro_tpl.macros));
      self.add_macros_from_template(tera, macro_tpl)?;
    }

    self
      .macro_template_map
      .insert(template_name, macro_namespace_map);

    for parent in &template.parents {
      let parent = &parent[..];
      let parent_template = tera.get_template(parent)?;
      self.add_macros_from_template(tera, parent_template);
    }

    Ok(())
  }

  #[inline]
  pub fn lookup_macro(
    &self,
    template_name: &'a str,
    macro_namespace: &'a str,
    macro_name: &'a str,
  ) -> Result<(&'a str, &'a MacroDefinition)> {
    match self
      .macro_template_map
      .get(template_name)
      .and_then(|namespace_map| namespace_map.get(macro_namespace))
      .and_then(|macro_definition_map| {
        let &(macro_template, macro_definition_map) = macro_definition_map;

        macro_definition_map
          .get(macro_name)
          .map(|md| (macro_template, md))
      }) {
      Some(result) => Ok(result),
      None => {
        bail!(format!(
          "Macro `({}:{})` not found in template `{}`",
          macro_namespace, macro_name, template_name
        ))
      }
    }
  }

  /// Takes the MacroCollection.
  ///
  /// Original `MacroCollection` has processed only top level file.
  /// More macros may have been added based on processing.
  /// This allows renderer to reuse any work.
  ///
  ///  * _return_ - The macro collection
  ///
  #[inline]
  pub fn take_macro_collection(self: &mut Self) -> MacroCollection<'a> {
    let macro_template_map =
      ::std::mem::replace(&mut self.macro_template_map, MacroTemplateMap::default());

    MacroCollection { macro_template_map }
  }
}
