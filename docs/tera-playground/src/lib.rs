mod utils;

use tera::{Context, Tera};
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn render(template: &str, context: String) -> Result<String, JsValue> {
    tera_render(template, &context).map_err(|e| e.to_string().into())
}

fn tera_render(template: &str, context: &str) -> Result<String, tera::Error> {
    let context = Context::from_value(serde_json::from_str(context)?)?;
    Tera::one_off(template, &context, true)
}
