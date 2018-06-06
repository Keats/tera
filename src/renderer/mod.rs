//! Responsible for rendering tera templates


pub use self::renderer::Renderer;

mod ast_processor;
mod call_stack;
mod context;
mod for_loop;
mod ref_or_owned;
mod renderer;
mod tera_macro;

#[cfg(test)]
mod tests;