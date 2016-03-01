#![allow(dead_code)]

#![cfg_attr(feature = "dev", allow(unstable_features))]
#![cfg_attr(feature = "dev", feature(plugin))]
#![cfg_attr(feature = "dev", plugin(clippy))]

#![feature(custom_derive, plugin)]
#![plugin(serde_macros)]
extern crate serde;
extern crate serde_json;
extern crate walkdir;

mod lexer;
mod nodes;
mod parser;
mod context;
mod render;
mod template;



// The actual api
// TODO: move it to another file?
use std::collections::BTreeMap;
use std::io::prelude::*;
use std::fs::File;

use walkdir::WalkDir;

// Re-export templates and context
pub use template::Template;
pub use context::Context;
pub use render::{RenderError};

#[derive(Debug)]
pub struct Tera {
    templates: BTreeMap<String, Template>,
}


// Wanted api:
// let mut tera = Tera::new("templates/");
// tera.register_filter(Capitalize);
// ^ the above can panic as it should be run in compile or first time
// ^ it will have run lexer + parser so we only need to render
// ...
// tera.render("dashboard/index.html", &someData) (-> Result<String>)


impl Tera {
    pub fn new(dir: &str) -> Tera {
        let mut templates = BTreeMap::new();

        // We are parsing all the templates on instantiation
        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            // We only care about actual files
            if path.is_file() {
                // We clean the filename by removing the dir given
                // to Tera so users don't have to prefix everytime
                let filepath = path.to_string_lossy().replace(dir, "");
                // we know the file exists so unwrap all the things
                let mut f = File::open(path).unwrap();
                let mut input = String::new();
                f.read_to_string(&mut input).unwrap();
                templates.insert(filepath.to_owned(), Template::new(&filepath, &input));
            }
        }

        Tera {
            templates: templates
        }
    }

    pub fn render(&self, template_name: &str, data: Context) -> Result<String, RenderError> {
        let template = self.templates.get(template_name).unwrap(); // TODO error handling

        template.render(data)
    }

    pub fn get_template(&self, template_name: &str) -> Option<&Template> {
        self.templates.get(template_name)
    }
}
