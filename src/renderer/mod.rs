mod square_brackets;
#[cfg(test)]
mod tests;

mod call_stack;
mod for_loop;
mod macros;
mod processor;
mod stack_frame;

use serde_json::value::Value;

use self::processor::Processor;
use errors::Result;
use template::Template;
use tera::Tera;

/// Given a `Tera` and reference to `Template` and a `Context`, renders text
#[derive(Debug)]
pub struct Renderer<'a> {
    /// Template to render
    template: &'a Template,
    /// Houses other templates, filters, global functions, etc
    tera: &'a Tera,
    /// Read-only context to be bound to template˝
    context: Value,
    /// If set rendering should be escaped
    should_escape: bool,
}

impl<'a> Renderer<'a> {
    /// Create a new `Renderer`
    #[inline]
    pub fn new(template: &'a Template, tera: &'a Tera, context: Value) -> Renderer<'a> {
        let should_escape = tera.autoescape_suffixes.iter().any(|ext| {
            // We prefer a `path` if set, otherwise use the `name`
            if let Some(ref p) = template.path {
                return p.ends_with(ext);
            }
            template.name.ends_with(ext)
        });

        Renderer { template, tera, context, should_escape }
    }

    /// Combines the context with the Template to generate the end result
    pub fn render(&self) -> Result<String> {
        let output;

        {
            let mut processor =
                Processor::new(self.template, self.tera, &self.context, self.should_escape);

            output = processor.render()?;
        }

        Ok(output)
    }
}
