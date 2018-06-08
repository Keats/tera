// --- module imports ---

extern crate stopwatch;

// --- module use statements ---

use self::stopwatch::Stopwatch;
use errors::{Result, ResultExt};
use parser::ast::{FilterSection, MacroDefinition, Node};
use renderer::ast_processor::{last_parent, AstProcessor};
use renderer::call_stack::CallStack;
use renderer::context::Context;
use renderer::ref_or_owned::RefOrOwned;
use renderer::tera_macro::{MacroCollection, MacroFile};
use serde_json::value::{to_value, Number, Value};
use std::collections::HashMap;
use template::Template;
use tera::Tera;
use utils::escape_html;

// --- module struct definitions ---

/// Given a `Tera` and reference to `Template` and a `Context`, renders text
#[derive(Debug)]
pub struct Renderer<'a> {
    /// Template to render
    template: &'a Template,
    /// Houses other templates, filters, global functions, etc
    tera: &'a Tera,
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
    pub fn new(
        template: &'a Template,
        tera: &'a Tera,
        context_value: Value,
    ) -> Result<Renderer<'a>> {
        let template_root = last_parent(tera, template).unwrap_or(template);

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
            macro_collection: MacroCollection::from_template_root(&template_root.name[..], tera)?,
            context_value,
            should_escape,
        })
    }

    /// Combines the context with the Template to generate text results
    ///
    ///  * _return_ - Generated text
    ///
    pub fn render(&mut self) -> Result<String> {
        let stopwatch = Stopwatch::start_new();

        let context_value = &self.context_value;
        let context = Context::from_value(context_value);
        let call_stack = CallStack::from_context(context, self.template);
        let mut rendering;

        {
            let mut ast_processor = AstProcessor::new(
                self.template,
                self.tera,
                call_stack,
                self.macro_collection.clone(),
                self.should_escape,
            );

            rendering = ast_processor.render_ast()?

            //.chain_err(|| format!("Failed to read template '{:?}'", path))?;
            //self.macro_collection = ast_processor.take_macro_collection();
        }

        let duration = stopwatch.elapsed_ms();

        println!(
            "Render (less_clone) for template took {}: For template ({}) with parents ({:?})",
            duration, self.template.name, self.template.parents
        );

        Ok(rendering)
    }
}

// --- module function definitions ---
