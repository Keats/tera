//! For processing macro invocations

// --- module use statements ---

use errors::{Result, ResultExt};
use parser::ast::{MacroDefinition};
use std::collections::HashMap;
use template::Template;
use tera::Tera;

// --- module type aliases ---

type MacroDefinitionMap<'a> = HashMap<& 'a str, & 'a MacroDefinition>;
type MacroFileMap<'a> = HashMap<& 'a str, MacroFile<'a>>;

// --- module struct definitions ---

/// Collection of all macro templates by file
#[derive(Clone, Debug, Default)]
pub struct MacroCollection<'a> {
  /// `MacroFiles` indexed by file name
  macro_files: MacroFileMap<'a>,
}
/// Implementation for type `MacroCollection`.
impl<'a> MacroCollection<'a> {
  /// Read macros from template file recursively
  ///
  ///  * `template_name` - Name of macro template file
  ///  * `tera` - Houses other templates, filters, global functions, etc
  ///  * _return_ - Macro definitions from a template file
  ///
  pub fn from_template_root(template_name: & 'a str,
      tera: & 'a Tera) -> Result<MacroCollection<'a>> {
    // custom <fn macro_collection_from_template_root>

    let mut macro_collection = MacroCollection {
      macro_files: MacroFileMap::new(),
    };

    macro_collection.add_macros_from_template(tera, tera.get_template(template_name)?)?;

    Ok(macro_collection)

    // end <fn macro_collection_from_template_root>
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
  pub fn add_macros_from_template(self: & mut Self,
      tera: & 'a Tera,
      template: & 'a Template) -> Result<()> {
    // custom <fn macro_collection_add_macros_from_template>

    info!(
      "Loading macros for {} at path {:?} with {} top level macros",
      template.name,
      template.path,
      template.macros.len()
    );

    for &(ref filename, ref namespace) in &template.imported_macro_files {
      info!(
        "\tLoading macros filename {}, namespace {}",
        filename, namespace
      );

      let macro_tpl = tera.get_template(filename)?;
      let mut macro_definitions = MacroDefinitionMap::new();
      for (macro_name, macro_definition) in macro_tpl.macros.iter() {
        macro_definitions.insert(&macro_name[..], macro_definition);
      }

      let filename_str = &filename[..];

      if !self.macro_files.contains_key(filename_str) {
        self.macro_files.insert(
          filename_str,
          MacroFile::from_definitions(filename_str, macro_definitions),
        );
      }

      if !macro_tpl.imported_macro_files.is_empty() {
        self.add_macros_from_template(tera, macro_tpl)?;
      }
    }

    Ok(())
    // end <fn macro_collection_add_macros_from_template>
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
  pub fn take_macro_collection(self: & mut Self) -> MacroCollection<'a> {
    // custom <fn macro_collection_take_macro_collection>

    let macro_files = ::std::mem::replace(&mut self.macro_files, MacroFileMap::default());
    MacroCollection { macro_files }

    // end <fn macro_collection_take_macro_collection>
  }

  // custom <impl macro_collection>
  // end <impl macro_collection>
}

/// The parsed macro definition and name of macro
#[derive(Clone, Debug)]
pub struct MacroFile<'a> {
  /// Name of macro
  file_name: & 'a str,
  /// Mapping of macro name to its definition
  macro_definitions: MacroDefinitionMap<'a>,
}
/// Implementation for type `MacroFile`.
impl<'a> MacroFile<'a> {
  /// Read macros from template file recursively
  ///
  ///  * `file_name` - Name of macro file
  ///  * `macro_definitions` - Macros contained in file>
  ///  * _return_ - Macro definitions from a template file
  ///
  pub fn from_definitions(file_name: & 'a str,
      macro_definitions: MacroDefinitionMap<'a>) -> MacroFile<'a> {
    MacroFile {
        file_name,
        macro_definitions
    }
  }

  // custom <impl macro_file>
  // end <impl macro_file>
}

