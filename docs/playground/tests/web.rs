//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use wasm_bindgen_test::*;

use playground::render;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn pass() {
    let template = "{{ greeting }} world!";
    let context = r#"{"greeting": "Hello"}"#.to_string();
    assert_eq!(render(template, context), Ok("Hello world!".to_string()));
}
