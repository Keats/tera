use std::collections::HashMap;
use std::io::prelude::*;
use std::fs::File;

use glob::glob;

use template::Template;
use filters::{FilterFn, string};
use context::Context;
use errors::{TeraResult, TeraError};
use render::Renderer;


#[derive(Debug)]
pub struct Tera {
    pub templates: HashMap<String, Template>,
    pub filters: HashMap<String, FilterFn>,
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

        let mut tera = Tera {
            templates: templates,
            filters: HashMap::new(),
        };
        tera.register_tera_filters();
        tera
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
    // Only for internal tests, do not use publicly
    pub fn add_template(&mut self, name: &str, content: &str) {
        self.templates.insert(name.to_string(), Template::new(name, content));
    }

    pub fn get_filter(&self, filter_name: &str) -> TeraResult<&FilterFn> {
        match self.filters.get(filter_name) {
            Some(fil) => Ok(fil),
            None => Err(TeraError::FilterNotFound(filter_name.to_string()))
        }
    }

    pub fn register_filter(&mut self, name: &str, filter: FilterFn) {
        self.filters.insert(name.to_string(), filter);
    }

    fn register_tera_filters(&mut self) {
        self.register_filter("upper", string::upper);
        self.register_filter("trim", string::trim);
        self.register_filter("truncate", string::truncate);
        self.register_filter("lower", string::lower);
        self.register_filter("wordcount", string::wordcount);
        self.register_filter("replace", string::replace);
    }
}

impl Default for Tera {
    fn default() -> Tera {
        let mut tera = Tera {
            templates: HashMap::new(),
            filters: HashMap::new(),
        };

        tera.register_tera_filters();
        tera
    }
}
