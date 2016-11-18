use std::collections::HashMap;
use std::io::prelude::*;
use std::fs::File;
use std::fmt;

use glob::glob;
use serde::Serialize;
use serde_json::value::to_value;

use template::Template;
use filters::{FilterFn, string, array, common, number};
use context::Context;
use errors::{TeraResult, TeraError};
use render::Renderer;
use testers::{self, TesterFn};


pub struct Tera {
    pub templates: HashMap<String, Template>,
    pub filters: HashMap<String, FilterFn>,
    pub testers: HashMap<String, TesterFn>,
    // Which extensions does Tera automatically autoescape on.
    // Defaults to [".html", ".htm", ".xml"]
    pub autoescape_extensions: Vec<&'static str>,
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
            testers: HashMap::new(),
            autoescape_extensions: vec![".html", ".htm", ".xml"]
        };

        tera.register_tera_filters();
        tera.register_tera_testers();
        tera
    }

    /// Renders a Tera template given a `Context`.
    pub fn render(&self, template_name: &str, data: Context) -> TeraResult<String> {
        let template = try!(self.get_template(template_name));
        let mut renderer = Renderer::new(template, self, data.as_json());

        renderer.render()
    }

    /// Renders a Tera template given a `Serializeable` object.
    pub fn value_render<T>(&self, template_name: &str, data: &T) -> TeraResult<String>
        where T: Serialize
    {
        let value = to_value(data);
        if !value.is_object() {
            return Err(TeraError::InvalidValue(template_name.to_string()))
        }

        let template = try!(self.get_template(template_name));
        let mut renderer = Renderer::new(template, self, value);
        renderer.render()
    }

    pub fn get_template(&self, template_name: &str) -> TeraResult<&Template> {
        match self.templates.get(template_name) {
            Some(tmpl) => Ok(tmpl),
            None => Err(TeraError::TemplateNotFound(template_name.to_string())),
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
            None => Err(TeraError::FilterNotFound(filter_name.to_string())),
        }
    }

    /// Register a filter with Tera.
    /// If a filter with that name already exists, it will be overwritten
    pub fn register_filter(&mut self, name: &str, filter: FilterFn) {
        self.filters.insert(name.to_string(), filter);
    }

    pub fn get_tester(&self, tester_name: &str) -> TeraResult<&TesterFn> {
        match self.testers.get(tester_name) {
            Some(t) => Ok(t),
            None => Err(TeraError::TesterNotFound(tester_name.to_string())),
        }
    }

    /// Register a tester with Tera.
    /// If a tester with that name already exists, it will be overwritten
    pub fn register_tester(&mut self, name: &str, tester: TesterFn) {
        self.testers.insert(name.to_string(), tester);
    }

    fn register_tera_filters(&mut self) {
        self.register_filter("upper", string::upper);
        self.register_filter("lower", string::lower);
        self.register_filter("trim", string::trim);
        self.register_filter("truncate", string::truncate);
        self.register_filter("wordcount", string::wordcount);
        self.register_filter("replace", string::replace);
        self.register_filter("capitalize", string::capitalize);
        self.register_filter("title", string::title);
        self.register_filter("striptags", string::striptags);
        self.register_filter("urlencode", string::urlencode);
        self.register_filter("escape", string::escape_html);

        self.register_filter("first", array::first);
        self.register_filter("last", array::last);
        self.register_filter("join", array::join);

        self.register_filter("pluralize", number::pluralize);
        self.register_filter("round", number::round);

        self.register_filter("length", common::length);
        self.register_filter("reverse", common::reverse);
    }

    fn register_tera_testers(&mut self) {
        self.register_tester("defined", testers::defined);
        self.register_tester("undefined", testers::undefined);
        self.register_tester("odd", testers::odd);
        self.register_tester("even", testers::even);
        self.register_tester("string", testers::string);
        self.register_tester("number", testers::number);
    }

    /// Select which extension(s) to automatically do HTML escaping on.
    /// Pass an empty vec to completely disable autoescape
    /// Note that autoescape will happen if the template name ends with one
    /// of the extensions given.
    /// Example: a file named `template.html` will be escaped by default but
    /// won't if you set pass `[".php.html"]` to that method.
    pub fn autoescape_on(&mut self, extensions: Vec<&'static str>) {
        self.autoescape_extensions = extensions;
    }
}

impl Default for Tera {
    fn default() -> Tera {
        let mut tera = Tera {
            templates: HashMap::new(),
            filters: HashMap::new(),
            testers: HashMap::new(),
            autoescape_extensions: vec![".html", ".htm", ".xml"]
        };

        tera.register_tera_filters();
        tera.register_tera_testers();
        tera
    }
}

// Needs a manual implementation since borrows in Fn's don't implement Debug.
impl fmt::Debug for Tera {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "Tera {}", "{"));
        for template in self.templates.keys() {
            try!(write!(f, "template={},", template));
        }

        for filter in self.filters.keys() {
            try!(write!(f, "filters={},", filter));
        }

        for tester in self.testers.keys() {
            try!(write!(f, "tester={},", tester));
        }

        write!(f, "{}", "}")
    }
}
