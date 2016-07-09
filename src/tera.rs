use std::collections::HashMap;
use std::io::prelude::*;
use std::fs::File;

use glob::glob;

use template::Template;
use context::Context;
use errors::{TeraResult, TeraError};
use render::Renderer;


#[derive(Debug)]
pub struct Tera {
    pub templates: HashMap<String, Template>,
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
                let filepath = path.to_string_lossy()
                    .replace("\\", "/") // change windows slash to forward slash
                    .replace(parent_dir, "");
                // we know the file exists so unwrap all the things
                let mut f = File::open(path).unwrap();
                let mut input = String::new();
                f.read_to_string(&mut input).unwrap();
                templates.insert(filepath.to_string(), Template::new(&filepath, &input));
            }
        }

        Tera {
            templates: templates
        }
    }

    pub fn render(&self, template_name: &str, data: Context) -> TeraResult<String> {
        let template = try!(self.get_template(template_name));
        let mut renderer = Renderer::new(template, self, data);

        renderer.render()
    }

    pub fn get_template(&self, template_name: &str) -> TeraResult<&Template> {
        match self.templates.get(template_name) {
            Some(tmpl) => Ok(tmpl),
            None => Err(TeraError::TemplateNotFound(template_name.to_string()))
        }
    }

    // Can panic!
    // Only for internal tests, do not use publicly for now
    pub fn add_template(&mut self, name: &str, content: &str) {
        self.templates.insert(name.to_string(), Template::new(name, content));
    }
}

impl Default for Tera {
    fn default() -> Tera {
        Tera {
            templates: HashMap::new()
        }
    }
}
