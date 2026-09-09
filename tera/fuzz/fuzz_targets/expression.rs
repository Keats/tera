#![no_main]
use libfuzzer_sys::fuzz_target;

extern crate tera;

use tera::Tera;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = String::from_utf8(data.to_vec()){
        let val = format!("{{{{ {} }}}}", s);
        std::thread::Builder::new()
            .stack_size(2048 * 1024)
            .spawn(move || {
                let mut tera = Tera::default();
                let _ = tera.add_raw_templates(vec![("fuzzing.html", val)]);
            })
            .unwrap()
            .join()
            .unwrap();
    }
});
