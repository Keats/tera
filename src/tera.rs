use std::collections::HashMap;
use std::io::prelude::*;
use std::fs::File;

use glob::glob;


use template::Template;
use context::Context;
use errors::TeraResult;
use errors;

#[derive(Debug)]
pub struct Tera {
    pub templates: HashMap<String, Template>,
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
                                 .replace("\\", "/")
                                 .replace(parent_dir, "");
                // we know the file exists so unwrap all the things
                let mut f = File::open(path).unwrap();
                let mut input = String::new();
                f.read_to_string(&mut input).unwrap();
                templates.insert(filepath.to_owned(), Template::new(&filepath, &input));
            }
        }

        // println!("templates {:?}", templates);

        Tera {
            templates: templates
        }
    }

    pub fn render(&self, template_name: &str, data: Context) -> TeraResult<String> {
        //let template = self.templates.get(template_name).unwrap(); // TODO error handling
        let template = match self.templates.get(template_name) {
            Some(tmpl) => tmpl,
            None => {
                println!("error in render {}", template_name);
                return Err(errors::template_not_found(template_name));
            }
        };

        // TODO: avoid cloning?
        template.render(data, self.templates.clone())
    }

    pub fn get_template(&self, template_name: &str) -> Option<&Template> {
        self.templates.get(template_name)
    }
}
