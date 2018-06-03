//! TODO: comment module renderer

// --- module imports ---

extern crate stopwatch;

// --- module use statements ---

use errors::{Result, ResultExt};
use parser::ast::{FilterSection, MacroDefinition, Node};
use renderer::ast_processor::AstProcessor;
use renderer::call_stack::CallStack;
use renderer::context::Context;
use renderer::ref_or_owned::RefOrOwned;
use renderer::tera_macro::{MacroCollection, MacroFile};
use self::stopwatch::Stopwatch;
use serde_json::value::{Number, Value, to_value};
use std::collections::HashMap;
use template::Template;
use tera::Tera;
use utils::escape_html;

// --- module struct definitions ---

/// Given a `Tera` and reference to `Template` and a `Context`, renders text
#[derive(Debug)]
pub struct Renderer<'a> {
  /// Template to render
  template: & 'a Template,
  /// Houses other templates, filters, global functions, etc
  tera: & 'a Tera,
  /// Parsed macro definitions
  macro_collection: MacroCollection<'a>,
  /// Read-only context to be bound to template˝
  context_value: Value,
  /// If set rendering should be escaped
  should_escape: bool,
}
/// Implementation for type `Renderer`.
impl<'a> Renderer<'a> {
  /// Create a new `Renderer`
  ///
  ///  * `template` - Template to generate
  ///  * `tera` - Tera struct containing other templates, filters, global functions etc
  ///  * `context_value` - Context to combine with template
  ///  * _return_ - Renderer ready `render`
  ///
  #[inline]
  pub fn new(template: & 'a Template,
      tera: & 'a Tera,
      context_value: Value) -> Result<Renderer<'a>> {

    let template = last_parent(tera, template).unwrap_or(template);

    let should_escape = tera.autoescape_suffixes.iter().any(|ext| {
        // We prefer a `path` if set, otherwise use the `name`
        if let Some(ref p) = template.path {
            return p.ends_with(ext);
        }
        template.name.ends_with(ext)
    });

    Ok(Renderer {
        template,
        tera,
        macro_collection: MacroCollection::from_template_root(
            &template.name[..],
            tera,
        )?,
        context_value,
        should_escape
    })
  }

  /// Combines the context with the Template to generate text results
  ///
  ///  * _return_ - Generated text
  ///
  pub fn render(& mut self) -> Result<String> {
        // custom <fn renderer_render>

        let stopwatch = Stopwatch::start_new();

        let context_value = &self.context_value;
        let context = Context::from_value(context_value);
        let call_stack = CallStack::from_context(context, self.template);

        let mut rendering;

        info!(
            "--- TERAx ---\n{:#?}\n---- Template ---- {:#?},\n---- Macros ----\n{:#?}\n---- Context ----\n{:#?}",
            self.tera, self.template, self.macro_collection, self.context_value
        );

        {
            let mut ast_processor = AstProcessor::new(
                self.tera,
                self.template,
                call_stack,
                self.macro_collection.clone(),
                self.should_escape,
            );
            rendering = ast_processor.render_ast(&self.template.ast)?;
            //self.macro_collection = ast_processor.take_macro_collection();
        }


        let duration = stopwatch.elapsed_ms();
        println!(
            "Render (less_clone) for template took {}: For template ({}) with parents ({:?})\n----- tera ------\n {:#?}",
            duration,
            self.template.name,
            self.template.parents,
            self.tera
        );      


        Ok(rendering)

        // end <fn renderer_render>
  }

    // custom <impl renderer>
    // end <impl renderer>
}

// --- module function definitions ---

/// Get last parent template
///
///  * `tera` - Tera that contains templates
///  * `template` - Template to find last template of
///  * _return_ - Last parent of template or `None`
///
#[inline]
pub fn last_parent<'a>(tera: & 'a Tera,
    template: & 'a Template) -> Option<& 'a Template> {
  template.parents.last().map(|parent| tera.get_template(parent).unwrap())
}

