use std::collections::HashMap;
use std::io::prelude::*;
use std::fs::File;
use std::fmt;
use std::path::Path;

use glob::glob;
use serde::Serialize;
use serde_json::value::to_value;

use template::Template;
use filters::{FilterFn, string, array, common, number};
use context::Context;
use errors::{Result, ResultExt};
use render::Renderer;
use testers::{self, TesterFn};


/// The main point of interaction in this library.
pub struct Tera {
    #[doc(hidden)]
    pub templates: HashMap<String, Template>,
    #[doc(hidden)]
    pub filters: HashMap<String, FilterFn>,
    #[doc(hidden)]
    pub testers: HashMap<String, TesterFn>,
    // Which extensions does Tera automatically autoescape on.
    // Defaults to [".html", ".htm", ".xml"]
    #[doc(hidden)]
    pub autoescape_extensions: Vec<&'static str>,
}


impl Tera {
    /// Create a new instance of Tera, containing all the parsed templates found
    /// in the `dir` glob.
    ///
    /// The example below is what the
    /// [compile_templates](macro.compile_templates.html) macros expands to.
    ///
    /// ```rust,ignore
    ///match Tera::new("templates/**/*") {
    ///    Ok(t) => t,
    ///    Err(e) => {
    ///        println!("Parsing error(s): {}", e);
    ///        ::std::process::exit(1);
    ///    }
    ///}
    /// ```
    pub fn new(dir: &str) -> Result<Tera> {
        if dir.find('*').is_none() {
            bail!("Tera expects a glob as input, no * were found in `{}`", dir);
        }

        let mut tera = Tera {
            templates: HashMap::new(),
            filters: HashMap::new(),
            testers: HashMap::new(),
            autoescape_extensions: vec![".html", ".htm", ".xml"]
        };

        // We are parsing all the templates on instantiation
        let mut errors = String::new();
        for entry in glob(dir).unwrap().filter_map(|e| e.ok()) {
            let path = entry.as_path();

            // Add every file to the tera instance.
            if path.is_file() {
                let parent_dir = dir.split_at(dir.find('*').unwrap()).0;
                let filepath = path.to_string_lossy()
                    .replace("\\", "/") // change windows slash to forward slash
                    .replace(parent_dir, "");

                if let Err(e) = tera.add_template_file_inner(&filepath, path) {
                    errors += &format!("\n* {}", e);
                    for e in e.iter().skip(1) {
                        errors += &format!("\n-- {}", e);
                    }
                }
            }
        }

        if !errors.is_empty() {
            bail!(errors)
        }

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
        // Recursive fn that finds all the parents and put them in an ordered Vec from closest to first parent
        // parent template
        fn build_chain(tera: &Tera, start: &Template, template: &Template, mut parents: Vec<String>) -> Result<Vec<String>> {
            if !parents.is_empty() && start.name == template.name {
                bail!("Circular extend detected for template '{}'. Inheritance chain: `{:?}`", start.name, parents);
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
                                "Template '{}' is inheriting from '{}', which doesn't exist or isn't loaded.",
                                template.name, p
                            );
                        }
                    }
                },
                None => Ok(parents)
            }
        }

        let mut templates = self.templates.clone();
        for template in self.templates.values() {
            // Simple template: no inheritance or blocks -> nothing to do
            if template.parent.is_none() && template.blocks.is_empty() {
                continue;
            }

            let mut tpl = template.clone();
            if tpl.parent.is_some() {
                tpl.parents = build_chain(self, template, template, vec![])?;
            }

            // Iterate over both blocks and templates and try to find the parents blocks
            // insert that into the tpl object once done so it's available directly in the template
            // without having to fetch all the parents to build it at runtime
            for (block_name, def) in &tpl.blocks {
                // push our own block first
                let mut definitions = vec![(tpl.name.clone(), def.clone())];

                // and then see if our parents have it
                for parent in &tpl.parents {
                    let t = self.get_template(parent)
                        .chain_err(|| format!("Couldn't find template {} while building inheritance chains", parent))?;

                    if let Some(b) = t.blocks.get(block_name) {
                        definitions.push((t.name.clone(), b.clone()));
                    }
                }
                tpl.blocks_definitions.insert(block_name.clone(), definitions);
            }
            templates.insert(template.name.clone(), tpl);
        }
        self.templates = templates;
        Ok(())
    }

    /// Renders a Tera template given a `Context` object.
    ///
    /// To render a template with an empty context, simply pass a new `Context` object
    ///
    /// ```rust,ignore
    /// // Rendering a template with an empty content
    /// tera.render("hello.html", Context::new());
    /// ```
    pub fn render(&self, template_name: &str, data: Context) -> Result<String> {
        let template = self.get_template(template_name)?;
        let mut renderer = Renderer::new(template, self, data.as_json());

        renderer.render()
    }

    /// Renders a Tera template given a `Serializeable` object.
    ///
    /// If `data` is not an object, an error will be returned.
    ///
    /// ```rust,ignore
    /// tera.render("hello.html", &user);
    /// ```
    pub fn value_render<T>(&self, template_name: &str, data: &T) -> Result<String>
        where T: Serialize
    {
        let value = to_value(data);
        if !value.is_object() {
            bail!(
                "Failed to value_render '{}': context isn't a JSON object. \
                The value passed needs to be a key-value object: struct, hashmap for example.",
                template_name
            );
        }

        let template = self.get_template(template_name)?;
        let mut renderer = Renderer::new(template, self, value);
        renderer.render()
    }

    /// Renders a one off template (for example a template coming from a user input)
    ///
    /// This creates a separate instance of Tera with no possibilities of adding custom filters
    /// or testers, parses the template and render it immediately.
    /// Any errors will mention the `one_off` template: this is the name given to the template by
    /// Tera
    ///
    /// ```rust,ignore
    /// let mut context = Context::new();
    /// context.add("greeting", &"hello");
    /// Tera::one_off("{{ greeting }} world", context);
    /// ```
    pub fn one_off(input: &str, data: Context, autoescape: bool) -> Result<String> {
        Tera::value_one_off(input, &data.as_json(), autoescape)
    }

    /// Renders a one off template (for example a template coming from a user input) given
    /// a `Serializeable` object.
    ///
    /// This creates a separate instance of Tera with no possibilities of adding custom filters
    /// or testers, parses the template and render it immediately.
    /// Any errors will mention the `one_off` template: this is the name given to the template by
    /// Tera
    ///
    /// ```rust,ignore
    /// Tera::value_one_off("{{ greeting }} world", &user);
    /// ```
    pub fn value_one_off<T>(input: &str, data: &T, autoescape: bool) -> Result<String>
        where T: Serialize
    {
        let mut tera = Tera::default();
        tera.add_template("one_off", input)?;
        if autoescape {
            tera.autoescape_on(vec!["one_off"]);
        }

        tera.value_render("one_off", data)
    }

    #[doc(hidden)]
    #[inline]
    pub fn get_template(&self, template_name: &str) -> Result<&Template> {
        match self.templates.get(template_name) {
            Some(tpl) => Ok(tpl),
            None => bail!("Template '{}' not found", template_name),
        }
    }

    /// Add a template to the Tera instance with the name `name` at the path
    /// `path`. Does not build the inheritance chains. This is an internal
    /// method. Return an error if the file at `path` could not be read or if
    /// the template was invalid.
    fn add_template_file_inner<P>(&mut self, name: &str, path: P) -> Result<()>
        where P: AsRef<Path>
    {
        // Read the template into a String.
        let path = path.as_ref();
        let mut f = File::open(path).chain_err(|| format!("Couldn't open template '{:?}'", path))?;
        let mut input = String::new();
        f.read_to_string(&mut input).chain_err(|| format!("Failed to read template '{:?}'", path))?;

        // Try to parse the template and insert it.
        let tpl = Template::new(name, &input, Some(path))
            .chain_err(|| format!("Failed to parse '{}'", name))?;
        self.templates.insert(name.to_string(), tpl);
        Ok(())
    }

    /// Add a template to the Tera instance with the name `name` at the path
    /// `path`.
    ///
    /// # Error
    ///
    /// If the file cannot be read, and error is returned. If the inheritance
    /// chain can't be built, such as adding a child template without the parent
    /// one, an error is returned. If you want to add several templates, use
    /// [Tera::add_templates](struct.Tera.html#method.add_template_files)
    ///
    /// ```rust,ignore
    /// tera.add_template("template_name", "/path/to/template.html");
    /// ```
    pub fn add_template_file<P: AsRef<Path>>(&mut self, name: &str, path: P) -> Result<()> {
        self.add_template_file_inner(name, path)?;
        self.build_inheritance_chains()?;
        Ok(())
    }

    /// Add all the templates given to the Tera instance.
    ///
    /// If the file cannot be read, and error is returned. If the inheritance
    /// chain can't be built, such as adding a child template without the parent
    /// one, an error is returned.
    ///
    /// ```rust,ignore
    /// tera.add_templates(vec![
    ///     ("template_name_1", "/path/to/first/template.html"),
    ///     ("second_template_name", "/path/to/second.html"),
    /// ]);
    /// ```
    pub fn add_template_files<P: AsRef<Path>>(&mut self, templates: Vec<(&str, P)>) -> Result<()> {
        for (name, path) in templates {
            self.add_template_file_inner(name, path)?;
        }

        self.build_inheritance_chains()?;
        Ok(())
    }

    /// Add a template to the Tera instance with the name `name` and contents
    /// `contents`.
    ///
    /// # Error
    ///
    /// If the inheritance chain can't be built, such as adding a child template
    /// without the parent one, an error is returned. If you want to add several
    /// templates, use
    /// [Tera::add_raw_templates](struct.Tera.html#method.add_templates)
    ///
    /// ```rust,ignore
    /// tera.add_template("template_name", "valid tera template contents");
    /// ```
    pub fn add_template<S: AsRef<str>>(&mut self, name: &str, contents: S) -> Result<()> {
        let template = Template::new(name, contents.as_ref(), None)
            .chain_err(|| format!("Failed to parse '{}'", name))?;

        self.templates.insert(name.to_string(), template);
        self.build_inheritance_chains()?;
        Ok(())
    }

    /// Add all of the `templates` to the Tera instance with the given names and
    /// contents.
    ///
    /// # Errors
    ///
    /// This will error if the inheritance chain can't be built, such as adding a child
    /// template without the parent one.
    ///
    /// ```rust,ignore
    /// tera.add_templates(vec![
    ///     ("template_name", "valid tera template contents"),
    ///     ("another_name", "more valid tera template contents"),
    /// ]);
    /// ```
    pub fn add_templates<S: AsRef<str>>(&mut self, templates: Vec<(&str, S)>) -> Result<()>  {
        for (name, content) in templates {
            let template = Template::new(name, content.as_ref(), None)
                .chain_err(|| format!("Failed to parse '{}'", name))?;

            self.templates.insert(name.to_string(), template);
        }

        self.build_inheritance_chains()?;
        Ok(())
    }

    #[doc(hidden)]
    #[inline]
    pub fn get_filter(&self, filter_name: &str) -> Result<&FilterFn> {
        match self.filters.get(filter_name) {
            Some(fil) => Ok(fil),
            None => bail!("Filter '{}' not found", filter_name),
        }
    }

    /// Register a filter with Tera.
    ///
    /// If a filter with that name already exists, it will be overwritten
    ///
    /// ```rust,ignore
    /// tera.register_filter("upper", string::upper);
    /// ```
    pub fn register_filter(&mut self, name: &str, filter: FilterFn) {
        self.filters.insert(name.to_string(), filter);
    }

    #[doc(hidden)]
    #[inline]
    pub fn get_tester(&self, tester_name: &str) -> Result<&TesterFn> {
        match self.testers.get(tester_name) {
            Some(t) => Ok(t),
            None => bail!("Tester '{}' not found", tester_name),
        }
    }

    /// Register a tester with Tera.
    ///
    /// If a tester with that name already exists, it will be overwritten
    ///
    /// ```rust,ignore
    /// tera.register_tester("odd", testers::odd);
    /// ```
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
        self.register_filter("slugify", string::slugify);
        self.register_filter("addslashes", string::addslashes);

        self.register_filter("first", array::first);
        self.register_filter("last", array::last);
        self.register_filter("join", array::join);

        self.register_filter("pluralize", number::pluralize);
        self.register_filter("round", number::round);
        self.register_filter("filesizeformat", number::filesizeformat);

        self.register_filter("length", common::length);
        self.register_filter("reverse", common::reverse);
        self.register_filter("date", common::date);
    }

    fn register_tera_testers(&mut self) {
        self.register_tester("defined", testers::defined);
        self.register_tester("undefined", testers::undefined);
        self.register_tester("odd", testers::odd);
        self.register_tester("even", testers::even);
        self.register_tester("string", testers::string);
        self.register_tester("number", testers::number);
        self.register_tester("divisibleby", testers::divisible_by);
        self.register_tester("iterable", testers::iterable);
    }

    /// Select which suffix(es) to automatically do HTML escaping on,
    ///`[".html", ".htm", ".xml"]` by default.
    ///
    /// Only call this function if you wish to change the defaults.
    ///
    ///
    /// ```rust,ignore
    /// // escape only files ending with `.php.html`
    /// tera.autoescape_on(vec![".php.html"]);
    /// // disable autoescaping completely
    /// tera.autoescape_on(vec![]);
    /// ```
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
    use context::Context;
    use serde_json::{Map as JsonObject, Value as JsonValue};

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
            "Template \'a\' is inheriting from \'b\', which doesn\'t exist or isn\'t loaded."
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

    #[test]
    fn test_can_autoescape_one_off_template() {
        let mut context = Context::new();
        context.add("greeting", &"<p>");
        let result = Tera::one_off("{{ greeting }} world", context, true).unwrap();

        assert_eq!(result, "&lt;p&gt; world");
    }

    #[test]
    fn test_can_disable_autoescape_one_off_template() {
        let mut context = Context::new();
        context.add("greeting", &"<p>");
        let result = Tera::one_off("{{ greeting }} world", context, false).unwrap();

        assert_eq!(result, "<p> world");
    }

    #[test]
    fn test_value_one_off_template() {
        let mut context = JsonObject::new();
        context.insert("greeting", JsonValue::String("Good morning".to_string()));
        let result = Tera::value_one_off("{{ greeting }} world", &context, true).unwrap();

        assert_eq!(result, "Good morning world");
    }
}
