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
use errors::{Result, ErrorKind};
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
    pub fn new(dir: &str) -> Result<Tera> {
        if dir.find('*').is_none() {
            bail!("Tera expects a glob as input, no * were found in {}", dir);
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
                templates.insert(filepath.to_string(), Template::new(&filepath, &input)?);
            }
        }

        let mut tera = Tera {
            templates: templates,
            filters: HashMap::new(),
            testers: HashMap::new(),
            autoescape_extensions: vec![".html", ".htm", ".xml"]
        };

        tera.build_inheritance_chains()?;
        tera.register_tera_filters();
        tera.register_tera_testers();
        Ok(tera)
    }

    // We need to know the hierarchy of templates to be able to render multiple extends level
    // This happens at compile to avoid checking it every time we want to render a template
    // This also checks for soundness issues in the inheritance chains, such as missing template or
    // circular extends.
    // It also builds the block inheritance chain and detects when super() is called in a place
    // where it can't possibly work
    fn build_inheritance_chains(&mut self) -> Result<()> {
        // Recursive fn that finds all the parents and put them in an ordered Vec from closest to main
        // parent template
        fn build_chain(tera: &Tera, start: &Template, template: &Template, mut parents: Vec<String>) -> Result<Vec<String>> {
            if parents.len() > 0 && start.name == template.name {
                bail!("Circular extend detected for template {:?}. Inheritance chain: {:?}", start.name, parents);
            }

            match template.parent {
                Some(ref p) => {
                    match tera.get_template(p) {
                        Ok(parent) => {
                            parents.push(parent.name.clone());
                            build_chain(tera, start, parent, parents)
                        },
                        Err(_) => {
                            bail!(
                                "Template {:?} is inheriting from {:?}, which doesn't exist or isn't loaded.",
                                template.name, p
                            );
                        }
                    }
                },
                None => Ok(parents)
            }
        }

        // TODO: Can we use iter_mut for the templates and modify in place?
        // If we do so, we run into a borrow issue since we need to pass the tera instance
        // to the build chain fn
        let mut templates = HashMap::new();
        for (_, template) in &self.templates {
            let mut tpl = template.clone();
            tpl.parents = build_chain(self, template, template, vec![])?;

            // TODO: iterate over both blocks and templates and try to find the parents blocks
            // insert that into the tpl object once done so it's available directly in the template
            // without having to fetch all the parents to build it at runtime
            for (block_name, def) in &tpl.blocks {
                // push our own block first
                let mut definitions = vec![(tpl.name.clone(), def.clone())];

                // and then see if our parents have it
                for parent in &tpl.parents {
                    let t = self.get_template(&parent).expect("Couldn't find template");
                    match t.blocks.get(block_name) {
                        Some(b) => definitions.push((t.name.clone(), b.clone())),
                        None => (),
                    };
                }
                tpl.blocks_definitions.insert(block_name.clone(), definitions);
            }
            templates.insert(tpl.name.clone(), tpl);
        }
        self.templates = templates;
        Ok(())
    }

    /// Renders a Tera template given a `Context`.
    pub fn render(&self, template_name: &str, data: Context) -> Result<String> {
        let template = self.get_template(template_name)?;
        let mut renderer = Renderer::new(template, self, data.as_json());

        renderer.render()
    }

    /// Renders a Tera template given a `Serializeable` object.
    pub fn value_render<T>(&self, template_name: &str, data: &T) -> Result<String>
        where T: Serialize
    {
        let value = to_value(data);
        if !value.is_object() {
            return Err(ErrorKind::InvalidValue(template_name.to_string()).into())
        }

        let template = self.get_template(template_name)?;
        let mut renderer = Renderer::new(template, self, value);
        renderer.render()
    }

    pub fn get_template(&self, template_name: &str) -> Result<&Template> {
        match self.templates.get(template_name) {
            Some(tmpl) => Ok(tmpl),
            None => Err(ErrorKind::TemplateNotFound(template_name.to_string()).into()),
        }
    }

    // Can panic!
    // Only for internal tests, do not use publicly
    #[doc(hidden)]
    pub fn add_template(&mut self, name: &str, content: &str) -> Result<()> {
        self.templates.insert(name.to_string(), Template::new(name, content)?);
        self.build_inheritance_chains()?;
        Ok(())
    }

    // Can panic!
    // Only for internal tests, do not use publicly
    #[doc(hidden)]
    pub fn add_templates(&mut self, templates: Vec<(&str, &str)>) -> Result<()>  {
        for (name, content) in templates {
            self.templates.insert(name.to_string(), Template::new(name, content)?);
        }
        self.build_inheritance_chains()?;
        Ok(())
    }

    pub fn get_filter(&self, filter_name: &str) -> Result<&FilterFn> {
        match self.filters.get(filter_name) {
            Some(fil) => Ok(fil),
            None => Err(ErrorKind::FilterNotFound(filter_name.to_string()).into()),
        }
    }

    /// Register a filter with Tera.
    /// If a filter with that name already exists, it will be overwritten
    pub fn register_filter(&mut self, name: &str, filter: FilterFn) {
        self.filters.insert(name.to_string(), filter);
    }

    pub fn get_tester(&self, tester_name: &str) -> Result<&TesterFn> {
        match self.testers.get(tester_name) {
            Some(t) => Ok(t),
            None => Err(ErrorKind::TesterNotFound(tester_name.to_string()).into()),
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
        write!(f, "Tera {}", "{")?;
        write!(f, "\n\ttemplates: [\n")?;

        for template in self.templates.keys() {
            writeln!(f, "\t\t{},", template)?;
        }
        write!(f, "\t]")?;
        write!(f, "\n\tfilters: [\n")?;

        for filter in self.filters.keys() {
            writeln!(f, "\t\t{},", filter)?;
        }
        write!(f, "\t]")?;
        write!(f, "\n\ttesters: [\n")?;

        for tester in self.testers.keys() {
            writeln!(f, "\t\t{},", tester)?;
        }
        write!(f, "\t]\n")?;

        writeln!(f, "{}", "}")
    }
}

#[cfg(test)]
mod tests {
    use super::{Tera};

    #[test]
    fn test_get_inheritance_chain() {
        let mut tera = Tera::default();
        tera.add_templates(vec![
            ("a", "{% extends \"b\" %}"),
            ("b", "{% extends \"c\" %}"),
            ("c", "{% extends \"d\" %}"),
            ("d", ""),
        ]).unwrap();

        assert_eq!(
            tera.get_template("a").unwrap().parents,
            vec!["b".to_string(), "c".to_string(), "d".to_string()]
        );

        assert_eq!(
            tera.get_template("b").unwrap().parents,
            vec!["c".to_string(), "d".to_string()]
        );

        assert_eq!(
            tera.get_template("c").unwrap().parents,
            vec!["d".to_string()]
        );

        assert_eq!(
            tera.get_template("d").unwrap().parents.len(),
            0
        );
    }

    #[test]
    fn test_missing_parent_template() {
        let mut tera = Tera::default();
        assert_eq!(
            tera.add_template("a", "{% extends \"b\" %}").unwrap_err().description(),
            "Template \"a\" is inheriting from \"b\", which doesn't exist or isn't loaded."
        );
    }

    #[test]
    fn test_circular_extends() {
        let mut tera = Tera::default();
        let err = tera.add_templates(vec![
            ("a", "{% extends \"b\" %}"),
            ("b", "{% extends \"a\" %}"),
        ]).unwrap_err();

        assert!(err.description().contains("Circular extend detected for template"));
    }

    #[test]
    fn test_get_parent_blocks_definition() {
        let mut tera = Tera::default();
        tera.add_templates(vec![
            ("grandparent", "{% block hey %}hello{% endblock hey %} {% block ending %}sincerely{% endblock ending %}"),
            ("parent", "{% extends \"grandparent\" %}{% block hey %}hi and grandma says {{ super() }}{% endblock hey %}"),
            ("child", "{% extends \"parent\" %}{% block hey %}dad says {{ super() }}{% endblock hey %}{% block ending %}{{ super() }} with love{% endblock ending %}"),
        ]).unwrap();

        let hey_definitions = tera.get_template("child").unwrap().blocks_definitions.get("hey").unwrap();
        assert_eq!(hey_definitions.len(), 3);
        let ending_definitions = tera.get_template("child").unwrap().blocks_definitions.get("ending").unwrap();
        assert_eq!(ending_definitions.len(), 2);
    }

    #[test]
    fn test_get_parent_blocks_definition_nested_block() {
        let mut tera = Tera::default();
        tera.add_templates(vec![
            ("grandparent", "{% block hey %}hello{% endblock hey %}"),
            ("parent", "{% extends \"grandparent\" %}{% block hey %}hi and grandma says {{ super() }} {% block ending %}sincerely{% endblock ending %}{% endblock hey %}"),
            ("child", "{% extends \"parent\" %}{% block hey %}dad says {{ super() }}{% endblock hey %}{% block ending %}{{ super() }} with love{% endblock ending %}"),
        ]).unwrap();

        let hey_definitions = tera.get_template("child").unwrap().blocks_definitions.get("hey").unwrap();
        assert_eq!(hey_definitions.len(), 3);
        let ending_definitions = tera.get_template("parent").unwrap().blocks_definitions.get("ending").unwrap();
        assert_eq!(ending_definitions.len(), 1);
    }
}
