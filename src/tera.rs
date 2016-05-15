use std::collections::HashMap;
use std::io::prelude::*;
use std::fs::File;

use glob::glob;
use serde_json::value::{Value as Json};


use template::Template;
use context::Context;
use errors::TeraResult;


pub type TeraFunction = fn(Vec<Json>, HashMap<String, Json>) -> TeraResult<String>;

#[derive(Debug)]
pub struct Tera {
    pub templates: HashMap<String, Template>,
    pub functions: HashMap<String, TeraFunction>
}

impl Tera {
    pub fn new(dir: &str) -> Tera {
        // TODO: add tests
        if dir.find('*').is_none() {
            panic!("Tera expects a glob as input, no * were found in {}", dir);
        }

        let mut templates = HashMap::new();

        // We are parsing all the templates on instantiation
        for entry in glob(dir).unwrap().filter_map(|e| e.ok()) {
            let path = entry.as_path();
            // We only care about actual files
            if path.is_file() {
                // We clean the filename by removing the dir given
                // to Tera so users don't have to prefix everytime
                let parent_dir = dir.split_at(dir.find('*').unwrap()).0;
                let filepath = path.to_string_lossy().replace(parent_dir, "");
                // we know the file exists so unwrap all the things
                let mut f = File::open(path).unwrap();
                let mut input = String::new();
                f.read_to_string(&mut input).unwrap();
                templates.insert(filepath.to_owned(), Template::new(&filepath, &input));
            }
        }

        Tera {
            templates: templates,
            functions: HashMap::new()
        }
    }

    pub fn render(&self, template_name: &str, data: Context) -> TeraResult<String> {
        // TODO error handling if template not found
        let template = self.templates.get(template_name).unwrap();
        template.render(data, self)
    }

    pub fn get_template(&self, template_name: &str) -> Option<&Template> {
        self.templates.get(template_name)
    }

    pub fn add_function(&mut self, name: &str, function: TeraFunction) {
        self.functions.insert(name.to_owned(), function);
    }

    fn get_function(&self, name: &str) -> Option<&TeraFunction> {
        self.functions.get(name)
    }
}
