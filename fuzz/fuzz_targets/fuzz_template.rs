#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate tera;

use tera::{Context, Tera};

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = String::from_utf8(data.to_vec()){
        let _ = Tera::one_off(&s, &Context::new(), true);
    }
});