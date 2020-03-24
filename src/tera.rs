use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::sync::Arc;

use globwalk::glob;

use crate::builtins::filters::{array, common, number, object, string, Filter};
use crate::builtins::functions::{self, Function};
use crate::builtins::testers::{self, Test};
use crate::context::Context;
use crate::errors::{Error, Result};
use crate::renderer::Renderer;
use crate::template::Template;
use crate::utils::escape_html;

/// The of the the template used for `Tera::render_str` and `Tera::one_off`.
const ONE_OFF_TEMPLATE_NAME: &str = "__tera_one_off";

/// The escape function type definition
pub type EscapeFn = fn(&str) -> String;

/// The main point of interaction in this library.
#[derive(Clone)]
pub struct Tera {
    // The glob used in `Tera::new`, None if Tera was instantiated differently
    #[doc(hidden)]
    glob: Option<String>,
    #[doc(hidden)]
    pub templates: HashMap<String, Template>,
    #[doc(hidden)]
    pub filters: HashMap<String, Arc<dyn Filter>>,
    #[doc(hidden)]
    pub testers: HashMap<String, Arc<dyn Test>>,
    #[doc(hidden)]
    pub functions: HashMap<String, Arc<dyn Function>>,
    // Which extensions does Tera automatically autoescape on.
    // Defaults to [".html", ".htm", ".xml"]
    #[doc(hidden)]
    pub autoescape_suffixes: Vec<&'static str>,
    #[doc(hidden)]
    escape_fn: EscapeFn,
}

impl Tera {
    fn create(dir: &str, parse_only: bool) -> Result<Tera> {
        if dir.find('*').is_none() {
            return Err(Error::msg(format!(
                "Tera expects a glob as input, no * were found in `{}`",
                dir
            )));
        }

        let mut tera = Tera {
            glob: Some(dir.to_string()),
            templates: HashMap::new(),
            filters: HashMap::new(),
            functions: HashMap::new(),
            testers: HashMap::new(),
            autoescape_suffixes: vec![".html", ".htm", ".xml"],
            escape_fn: escape_html,
        };

        tera.load_from_glob()?;
        if !parse_only {
            tera.build_inheritance_chains()?;
            tera.check_macro_files()?;
        }
        tera.register_tera_filters();
        tera.register_tera_testers();
        tera.register_tera_functions();
        Ok(tera)
    }

    /// Create a new instance of Tera, containing all the parsed templates found in the `dir` glob
    ///
    ///```ignore
    ///match Tera::new("templates/**/*") {
    ///    Ok(t) => t,
    ///    Err(e) => {
    ///        println!("Parsing error(s): {}", e);
    ///        ::std::process::exit(1);
    ///    }
    ///}
    ///```
    pub fn new(dir: &str) -> Result<Tera> {
        Self::create(dir, false)
    }

    /// Create a new instance of Tera, containing all the parsed templates found in the `dir` glob
    /// The difference with `Tera::new` is that it won't build the inheritance chains automatically
    /// so you are free to modify the templates if you need to.
    /// You will NOT get a working Tera instance using `Tera::parse`, you will need to call
    /// `tera.build_inheritance_chains()` to make it usable
    ///
    ///```ignore
    ///let mut tera = match Tera::parse("templates/**/*") {
    ///    Ok(t) => t,
    ///    Err(e) => {
    ///        println!("Parsing error(s): {}", e);
    ///        ::std::process::exit(1);
    ///    }
    ///};
    ///tera.build_inheritance_chains()?;
    ///```
    pub fn parse(dir: &str) -> Result<Tera> {
        Self::create(dir, true)
    }

    /// Loads all the templates found in the glob that was given to Tera::new
    fn load_from_glob(&mut self) -> Result<()> {
        if self.glob.is_none() {
            return Err(Error::msg("Tera can only load from glob if a glob is provided"));
        }
        // We want to preserve templates that have been added through
        // Tera::extend so we only keep those
        self.templates = self
            .templates
            .iter()
            .filter(|&(_, t)| t.from_extend)
            .map(|(n, t)| (n.clone(), t.clone())) // TODO: avoid that clone
            .collect();

        let mut errors = String::new();

        let dir = self.glob.clone().unwrap();
        // We clean the filename by removing the dir given
        // to Tera so users don't have to prefix everytime
        let mut parent_dir = dir.split_at(dir.find('*').unwrap()).0;
        // Remove `./` from the glob if used as it would cause an error in strip_prefix
        if parent_dir.starts_with("./") {
            parent_dir = &parent_dir[2..];
        }

        // We are parsing all the templates on instantiation
        for entry in glob(&dir).unwrap().filter_map(std::result::Result::ok) {
            let mut path = entry.into_path();
            // We only care about actual files
            if path.is_file() {
                if path.starts_with("./") {
                    path = path.strip_prefix("./").unwrap().to_path_buf();
                }

                let filepath = path
                    .strip_prefix(&parent_dir)
                    .unwrap()
                    .to_string_lossy()
                    // unify on forward slash
                    .replace("\\", "/");

                if let Err(e) = self.add_file(Some(&filepath), path) {
                    use std::error::Error;

                    errors += &format!("\n* {}", e);
                    let mut cause = e.source();
                    while let Some(e) = cause {
                        errors += &format!("\n{}", e);
                        cause = e.source();
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(Error::msg(errors));
        }

        Ok(())
    }

    // Add a template from a path: reads the file and parses it.
    // This will return an error if the template is invalid and doesn't check the validity of
    // inheritance chains.
    fn add_file<P: AsRef<Path>>(&mut self, name: Option<&str>, path: P) -> Result<()> {
        let path = path.as_ref();
        let tpl_name = name.unwrap_or_else(|| path.to_str().unwrap());

        let mut f = File::open(path)
            .map_err(|e| Error::chain(format!("Couldn't open template '{:?}'", path), e))?;

        let mut input = String::new();
        f.read_to_string(&mut input)
            .map_err(|e| Error::chain(format!("Failed to read template '{:?}'", path), e))?;

        let tpl = Template::new(tpl_name, Some(path.to_str().unwrap().to_string()), &input)
            .map_err(|e| Error::chain(format!("Failed to parse {:?}", path), e))?;

        self.templates.insert(tpl_name.to_string(), tpl);
        Ok(())
    }

    /// We need to know the hierarchy of templates to be able to render multiple extends level
    /// This happens at compile-time to avoid checking it every time we want to render a template
    /// This also checks for soundness issues in the inheritance chains, such as missing template or
    /// circular extends.
    /// It also builds the block inheritance chain and detects when super() is called in a place
    /// where it can't possibly work
    ///
    /// You generally don't need to call that yourself, unless you used `Tera::parse`
    pub fn build_inheritance_chains(&mut self) -> Result<()> {
        // Recursive fn that finds all the parents and put them in an ordered Vec from closest to first parent
        // parent template
        fn build_chain(
            templates: &HashMap<String, Template>,
            start: &Template,
            template: &Template,
            mut parents: Vec<String>,
        ) -> Result<Vec<String>> {
            if !parents.is_empty() && start.name == template.name {
                return Err(Error::circular_extend(&start.name, parents));
            }

            match template.parent {
                Some(ref p) => match templates.get(p) {
                    Some(parent) => {
                        parents.push(parent.name.clone());
                        build_chain(templates, start, parent, parents)
                    }
                    None => Err(Error::missing_parent(&template.name, &p)),
                },
                None => Ok(parents),
            }
        }

        // TODO: if we can rewrite the 2 loops below to be only one loop, that'd be great
        let mut tpl_parents = HashMap::new();
        let mut tpl_block_definitions = HashMap::new();
        for (name, template) in &self.templates {
            if template.parent.is_none() && template.blocks.is_empty() {
                continue;
            }

            let parents = build_chain(&self.templates, template, template, vec![])?;

            let mut blocks_definitions = HashMap::new();
            for (block_name, def) in &template.blocks {
                // push our own block first
                let mut definitions = vec![(template.name.clone(), def.clone())];

                // and then see if our parents have it
                for parent in &parents {
                    let t = self.get_template(parent)?;

                    if let Some(b) = t.blocks.get(block_name) {
                        definitions.push((t.name.clone(), b.clone()));
                    }
                }
                blocks_definitions.insert(block_name.clone(), definitions);
            }
            tpl_parents.insert(name.clone(), parents);
            tpl_block_definitions.insert(name.clone(), blocks_definitions);
        }

        for template in self.templates.values_mut() {
            // Simple template: no inheritance or blocks -> nothing to do
            if template.parent.is_none() && template.blocks.is_empty() {
                continue;
            }

            template.parents = match tpl_parents.remove(&template.name) {
                Some(parents) => parents,
                None => vec![],
            };
            template.blocks_definitions = match tpl_block_definitions.remove(&template.name) {
                Some(blocks) => blocks,
                None => HashMap::new(),
            };
        }

        Ok(())
    }

    /// We keep track of macro files loaded in each Template so we can know whether one or them
    /// is missing and error accordingly before the user tries to render a template.
    ///
    /// As with `self::build_inheritance_chains`, you don't usually need to call that yourself.
    pub fn check_macro_files(&self) -> Result<()> {
        for template in self.templates.values() {
            for &(ref tpl_name, _) in &template.imported_macro_files {
                if !self.templates.contains_key(tpl_name) {
                    return Err(Error::msg(format!(
                        "Template `{}` loads macros from `{}` which isn't present in Tera",
                        template.name, tpl_name
                    )));
                }
            }
        }

        Ok(())
    }

    /// Renders a Tera template given a `tera::Context`,
    ///
    /// To render a template with an empty context, simply pass a new `tera::Context` object
    ///
    /// ```rust,ignore
    /// // Rendering a template with a normal context
    /// let mut context = Context::new();
    /// context.insert("age", 18);
    /// tera.render("hello.html", context);
    /// // Rendering a template with an empty context
    /// tera.render("hello.html", Context::new());
    /// ```
    pub fn render(&self, template_name: &str, context: &Context) -> Result<String> {
        let template = self.get_template(template_name)?;
        let renderer = Renderer::new(template, self, context);
        renderer.render()
    }

    /// Renders a one off template (for example a template coming from a user
    /// input) given a `Context` and an instance of Tera. This allows you to
    /// render templates using custom filters or functions.
    ///
    /// Any errors will mention the `__tera_one_off` template: this is the name
    /// given to the template by Tera.
    ///
    /// ```rust,ignore
    /// let mut context = Context::new();
    /// context.insert("greeting", &"Hello");
    /// let string = tera.render_str("{{ greeting }} World!", &context)?;
    /// assert_eq!(string, "Hello World!");
    /// ```
    pub fn render_str(&mut self, input: &str, context: &Context) -> Result<String> {
        self.add_raw_template(ONE_OFF_TEMPLATE_NAME, input)?;
        let result = self.render(ONE_OFF_TEMPLATE_NAME, &context);
        self.templates.remove(ONE_OFF_TEMPLATE_NAME);
        result
    }

    /// Renders a one off template (for example a template coming from a user input) given a `Context`
    ///
    /// This creates a separate instance of Tera with no possibilities of adding custom filters
    /// or testers, parses the template and render it immediately.
    /// Any errors will mention the `__tera_one_off` template: this is the name given to the template by
    /// Tera
    ///
    /// ```rust,ignore
    /// let mut context = Context::new();
    /// context.insert("greeting", &"hello");
    /// Tera::one_off("{{ greeting }} world", &context, true);
    /// ```
    pub fn one_off(input: &str, context: &Context, autoescape: bool) -> Result<String> {
        let mut tera = Tera::default();

        if autoescape {
            tera.autoescape_on(vec![ONE_OFF_TEMPLATE_NAME]);
        }

        tera.render_str(input, context)
    }

    #[doc(hidden)]
    #[inline]
    pub fn get_template(&self, template_name: &str) -> Result<&Template> {
        match self.templates.get(template_name) {
            Some(tpl) => Ok(tpl),
            None => Err(Error::template_not_found(template_name)),
        }
    }

    /// Add a single template to the Tera instance
    ///
    /// This will error if the inheritance chain can't be built, such as adding a child
    /// template without the parent one.
    /// If you want to add several templates, use [Tera::add_templates](struct.Tera.html#method.add_raw_templates)
    ///
    /// ```rust,ignore
    /// tera.add_raw_template("new.html", "Blabla");
    /// ```
    pub fn add_raw_template(&mut self, name: &str, content: &str) -> Result<()> {
        let tpl = Template::new(name, None, content)
            .map_err(|e| Error::chain(format!("Failed to parse '{}'", name), e))?;
        self.templates.insert(name.to_string(), tpl);
        self.build_inheritance_chains()?;
        self.check_macro_files()?;
        Ok(())
    }

    /// Add all the templates given to the Tera instance
    ///
    /// This will error if the inheritance chain can't be built, such as adding a child
    /// template without the parent one.
    ///
    /// ```rust,ignore
    /// tera.add_raw_templates(vec![
    ///     ("new.html", "blabla"),
    ///     ("new2.html", "hello"),
    /// ]);
    /// ```
    pub fn add_raw_templates(&mut self, templates: Vec<(&str, &str)>) -> Result<()> {
        for (name, content) in templates {
            let tpl = Template::new(name, None, content)
                .map_err(|e| Error::chain(format!("Failed to parse '{}'", name), e))?;
            self.templates.insert(name.to_string(), tpl);
        }
        self.build_inheritance_chains()?;
        self.check_macro_files()?;
        Ok(())
    }

    /// Add a single template from a path to the Tera instance. The default name for the template is
    /// the path given, but this can be renamed with the `name` parameter
    ///
    /// This will error if the inheritance chain can't be built, such as adding a child
    /// template without the parent one.
    /// If you want to add several file, use [Tera::add_template_files](struct.Tera.html#method.add_template_files)
    ///
    /// ```rust,ignore
    /// // Use path as name
    /// tera.add_template_file(path, None);
    /// // Rename
    /// tera.add_template_file(path, Some("index"));
    /// ```
    pub fn add_template_file<P: AsRef<Path>>(&mut self, path: P, name: Option<&str>) -> Result<()> {
        self.add_file(name, path)?;
        self.build_inheritance_chains()?;
        self.check_macro_files()?;
        Ok(())
    }

    /// Add several templates from paths to the Tera instance. The default name for the template is
    /// the path given, but this can be renamed with the second parameter of the tuple
    ///
    /// This will error if the inheritance chain can't be built, such as adding a child
    /// template without the parent one.
    ///
    /// ```rust,ignore
    /// tera.add_template_files(vec![
    ///     (path1, None), // this template will have the value of path1 as name
    ///     (path2, Some("hey")), // this template will have `hey` as name
    /// ]);
    /// ```
    pub fn add_template_files<P: AsRef<Path>>(
        &mut self,
        files: Vec<(P, Option<&str>)>,
    ) -> Result<()> {
        for (path, name) in files {
            self.add_file(name, path)?;
        }
        self.build_inheritance_chains()?;
        self.check_macro_files()?;
        Ok(())
    }

    #[doc(hidden)]
    #[inline]
    pub fn get_filter(&self, filter_name: &str) -> Result<&dyn Filter> {
        match self.filters.get(filter_name) {
            Some(fil) => Ok(&**fil),
            None => Err(Error::filter_not_found(filter_name)),
        }
    }

    /// Register a filter with Tera.
    ///
    /// If a filter with that name already exists, it will be overwritten
    ///
    /// ```rust,ignore
    /// tera.register_filter("upper", string::upper);
    /// ```
    pub fn register_filter<F: Filter + 'static>(&mut self, name: &str, filter: F) {
        self.filters.insert(name.to_string(), Arc::new(filter));
    }

    #[doc(hidden)]
    #[inline]
    pub fn get_tester(&self, tester_name: &str) -> Result<&dyn Test> {
        match self.testers.get(tester_name) {
            Some(t) => Ok(&**t),
            None => Err(Error::test_not_found(tester_name)),
        }
    }

    /// Register a tester with Tera.
    ///
    /// If a tester with that name already exists, it will be overwritten
    ///
    /// ```rust,ignore
    /// tera.register_tester("odd", testers::odd);
    /// ```
    pub fn register_tester<T: Test + 'static>(&mut self, name: &str, tester: T) {
        self.testers.insert(name.to_string(), Arc::new(tester));
    }

    #[doc(hidden)]
    #[inline]
    pub fn get_function(&self, fn_name: &str) -> Result<&dyn Function> {
        match self.functions.get(fn_name) {
            Some(t) => Ok(&**t),
            None => Err(Error::function_not_found(fn_name)),
        }
    }

    /// Register a function with Tera.
    ///
    /// If a function with that name already exists, it will be overwritten
    ///
    /// ```rust,ignore
    /// tera.register_function("range", range);
    /// ```
    pub fn register_function<F: Function + 'static>(&mut self, name: &str, function: F) {
        self.functions.insert(name.to_string(), Arc::new(function));
    }

    fn register_tera_filters(&mut self) {
        self.register_filter("upper", string::upper);
        self.register_filter("lower", string::lower);
        self.register_filter("trim", string::trim);
        self.register_filter("trim_start", string::trim_start);
        self.register_filter("trim_end", string::trim_end);
        self.register_filter("trim_start_matches", string::trim_start_matches);
        self.register_filter("trim_end_matches", string::trim_end_matches);
        #[cfg(feature = "builtins")]
        self.register_filter("truncate", string::truncate);
        self.register_filter("wordcount", string::wordcount);
        self.register_filter("replace", string::replace);
        self.register_filter("capitalize", string::capitalize);
        self.register_filter("title", string::title);
        self.register_filter("striptags", string::striptags);
        #[cfg(feature = "builtins")]
        self.register_filter("urlencode", string::urlencode);
        self.register_filter("escape", string::escape_html);
        self.register_filter("escape_xml", string::escape_xml);
        #[cfg(feature = "builtins")]
        self.register_filter("slugify", string::slugify);
        self.register_filter("addslashes", string::addslashes);
        self.register_filter("split", string::split);
        self.register_filter("int", string::int);
        self.register_filter("float", string::float);

        self.register_filter("first", array::first);
        self.register_filter("last", array::last);
        self.register_filter("nth", array::nth);
        self.register_filter("join", array::join);
        self.register_filter("sort", array::sort);
        self.register_filter("unique", array::unique);
        self.register_filter("slice", array::slice);
        self.register_filter("group_by", array::group_by);
        self.register_filter("filter", array::filter);
        self.register_filter("map", array::map);
        self.register_filter("concat", array::concat);

        self.register_filter("pluralize", number::pluralize);
        self.register_filter("round", number::round);

        #[cfg(feature = "builtins")]
        self.register_filter("filesizeformat", number::filesizeformat);

        self.register_filter("length", common::length);
        self.register_filter("reverse", common::reverse);
        #[cfg(feature = "builtins")]
        self.register_filter("date", common::date);
        self.register_filter("json_encode", common::json_encode);
        self.register_filter("as_str", common::as_str);

        self.register_filter("get", object::get);
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
        self.register_tester("object", testers::object);
        self.register_tester("starting_with", testers::starting_with);
        self.register_tester("ending_with", testers::ending_with);
        self.register_tester("containing", testers::containing);
        self.register_tester("matching", testers::matching);
    }

    fn register_tera_functions(&mut self) {
        self.register_function("range", functions::range);
        #[cfg(feature = "builtins")]
        self.register_function("now", functions::now);
        self.register_function("throw", functions::throw);
        #[cfg(feature = "builtins")]
        self.register_function("get_random", functions::get_random);
        self.register_function("get_env", functions::get_env);
    }

    /// Select which suffix(es) to automatically do HTML escaping on,
    ///`[".html", ".htm", ".xml"]` by default.
    ///
    /// Only call this function if you wish to change the defaults.
    ///
    ///
    ///```ignore
    /// // escape only files ending with `.php.html`
    /// tera.autoescape_on(vec![".php.html"]);
    /// // disable autoescaping completely
    /// tera.autoescape_on(vec![]);
    ///```
    pub fn autoescape_on(&mut self, suffixes: Vec<&'static str>) {
        self.autoescape_suffixes = suffixes;
    }

    #[doc(hidden)]
    #[inline]
    pub fn get_escape_fn(&self) -> &EscapeFn {
        &self.escape_fn
    }

    /// Set user-defined function which applied to a rendered content.
    ///
    ///```rust,ignore
    /// fn escape_c_string(input: &str) -> String { ... }
    ///
    /// // make valid C string literal
    /// tera.set_escape_fn(escape_c_string);
    /// tera.add_raw_template("foo", "\"{{ content }}\"").unwrap();
    /// tera.autoescape_on(vec!["foo"]);
    /// let mut context = Context::new();
    /// context.insert("content", &"Hello\n\'world\"!");
    /// let result = tera.render("foo", &context).unwrap();
    /// assert_eq!(result, r#""Hello\n\'world\"!""#);
    ///```
    pub fn set_escape_fn(&mut self, function: EscapeFn) {
        self.escape_fn = function;
    }

    /// Reset escape function to default `tera::escape_html`.
    pub fn reset_escape_fn(&mut self) {
        self.escape_fn = escape_html;
    }

    /// Re-parse all templates found in the glob given to Tera
    /// Use this when you are watching a directory and want to reload everything,
    /// for example when a file is added.
    ///
    /// If you are adding templates without using a glob, we can't know when a template
    /// is deleted, which would result in an error if we are trying to reload that file
    pub fn full_reload(&mut self) -> Result<()> {
        if self.glob.is_some() {
            self.load_from_glob()?;
        } else {
            return Err(Error::msg("Reloading is only available if you are using a glob"));
        }

        self.build_inheritance_chains()?;
        self.check_macro_files()
    }

    /// Use that method when you want to add a given Tera instance templates/filters/testers
    /// to your own. If a template/filter/tester with the same name already exists in your instance,
    /// it will not be overwritten.
    ///
    ///```rust,ignore
    /// // add all the templates from FRAMEWORK_TERA
    /// // except the ones that have an identical name to the ones in `my_tera`
    /// my_tera.extend(&FRAMEWORK_TERA);
    ///```
    pub fn extend(&mut self, other: &Tera) -> Result<()> {
        for (name, template) in &other.templates {
            if !self.templates.contains_key(name) {
                let mut tpl = template.clone();
                tpl.from_extend = true;
                self.templates.insert(name.to_string(), tpl);
            }
        }

        for (name, filter) in &other.filters {
            if !self.filters.contains_key(name) {
                self.filters.insert(name.to_string(), filter.clone());
            }
        }

        for (name, tester) in &other.testers {
            if !self.testers.contains_key(name) {
                self.testers.insert(name.to_string(), tester.clone());
            }
        }

        self.build_inheritance_chains()?;
        self.check_macro_files()
    }
}

impl Default for Tera {
    fn default() -> Tera {
        let mut tera = Tera {
            glob: None,
            templates: HashMap::new(),
            filters: HashMap::new(),
            testers: HashMap::new(),
            functions: HashMap::new(),
            autoescape_suffixes: vec![".html", ".htm", ".xml"],
            escape_fn: escape_html,
        };

        tera.register_tera_filters();
        tera.register_tera_testers();
        tera.register_tera_functions();
        tera
    }
}

// Needs a manual implementation since borrows in Fn's don't implement Debug.
impl fmt::Debug for Tera {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Tera {{")?;
        writeln!(f, "\n\ttemplates: [")?;

        for template in self.templates.keys() {
            writeln!(f, "\t\t{},", template)?;
        }
        write!(f, "\t]")?;
        writeln!(f, "\n\tfilters: [")?;

        for filter in self.filters.keys() {
            writeln!(f, "\t\t{},", filter)?;
        }
        write!(f, "\t]")?;
        writeln!(f, "\n\ttesters: [")?;

        for tester in self.testers.keys() {
            writeln!(f, "\t\t{},", tester)?;
        }
        writeln!(f, "\t]")?;

        writeln!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use std::collections::HashMap;
    use std::fs::File;

    use super::Tera;
    use crate::context::Context;
    use serde_json::{json, Value as JsonValue};

    #[test]
    fn test_get_inheritance_chain() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("a", "{% extends \"b\" %}"),
            ("b", "{% extends \"c\" %}"),
            ("c", "{% extends \"d\" %}"),
            ("d", ""),
        ])
        .unwrap();

        assert_eq!(
            tera.get_template("a").unwrap().parents,
            vec!["b".to_string(), "c".to_string(), "d".to_string()]
        );
        assert_eq!(tera.get_template("b").unwrap().parents, vec!["c".to_string(), "d".to_string()]);
        assert_eq!(tera.get_template("c").unwrap().parents, vec!["d".to_string()]);
        assert_eq!(tera.get_template("d").unwrap().parents.len(), 0);
    }

    #[test]
    fn test_missing_parent_template() {
        let mut tera = Tera::default();
        assert_eq!(
            tera.add_raw_template("a", "{% extends \"b\" %}").unwrap_err().to_string(),
            "Template \'a\' is inheriting from \'b\', which doesn\'t exist or isn\'t loaded."
        );
    }

    #[test]
    fn test_circular_extends() {
        let mut tera = Tera::default();
        let err = tera
            .add_raw_templates(vec![("a", "{% extends \"b\" %}"), ("b", "{% extends \"a\" %}")])
            .unwrap_err();

        assert!(err.to_string().contains("Circular extend detected for template"));
    }

    #[test]
    fn test_get_parent_blocks_definition() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            (
                "grandparent",
                "{% block hey %}hello{% endblock hey %} {% block ending %}sincerely{% endblock ending %}",
            ),
            (
                "parent",
                "{% extends \"grandparent\" %}{% block hey %}hi and grandma says {{ super() }}{% endblock hey %}",
            ),
            (
                "child",
                "{% extends \"parent\" %}{% block hey %}dad says {{ super() }}{% endblock hey %}{% block ending %}{{ super() }} with love{% endblock ending %}",
            ),
        ]).unwrap();

        let hey_definitions =
            tera.get_template("child").unwrap().blocks_definitions.get("hey").unwrap();
        assert_eq!(hey_definitions.len(), 3);

        let ending_definitions =
            tera.get_template("child").unwrap().blocks_definitions.get("ending").unwrap();
        assert_eq!(ending_definitions.len(), 2);
    }

    #[test]
    fn test_get_parent_blocks_definition_nested_block() {
        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("grandparent", "{% block hey %}hello{% endblock hey %}"),
            (
                "parent",
                "{% extends \"grandparent\" %}{% block hey %}hi and grandma says {{ super() }} {% block ending %}sincerely{% endblock ending %}{% endblock hey %}",
            ),
            (
                "child",
                "{% extends \"parent\" %}{% block hey %}dad says {{ super() }}{% endblock hey %}{% block ending %}{{ super() }} with love{% endblock ending %}",
            ),
        ]).unwrap();

        let hey_definitions =
            tera.get_template("child").unwrap().blocks_definitions.get("hey").unwrap();
        assert_eq!(hey_definitions.len(), 3);

        let ending_definitions =
            tera.get_template("parent").unwrap().blocks_definitions.get("ending").unwrap();
        assert_eq!(ending_definitions.len(), 1);
    }

    #[test]
    fn test_can_autoescape_one_off_template() {
        let mut context = Context::new();
        context.insert("greeting", &"<p>");
        let result = Tera::one_off("{{ greeting }} world", &context, true).unwrap();

        assert_eq!(result, "&lt;p&gt; world");
    }

    #[test]
    fn test_can_disable_autoescape_one_off_template() {
        let mut context = Context::new();
        context.insert("greeting", &"<p>");
        let result = Tera::one_off("{{ greeting }} world", &context, false).unwrap();

        assert_eq!(result, "<p> world");
    }

    #[test]
    fn test_set_escape_function() {
        let escape_c_string: super::EscapeFn = |input| {
            let mut output = String::with_capacity(input.len() * 2);
            for c in input.chars() {
                match c {
                    '\'' => output.push_str("\\'"),
                    '\"' => output.push_str("\\\""),
                    '\\' => output.push_str("\\\\"),
                    '\n' => output.push_str("\\n"),
                    '\r' => output.push_str("\\r"),
                    '\t' => output.push_str("\\t"),
                    _ => output.push(c),
                }
            }
            output
        };
        let mut tera = Tera::default();
        tera.add_raw_template("foo", "\"{{ content }}\"").unwrap();
        tera.autoescape_on(vec!["foo"]);
        tera.set_escape_fn(escape_c_string);
        let mut context = Context::new();
        context.insert("content", &"Hello\n\'world\"!");
        let result = tera.render("foo", &context).unwrap();
        assert_eq!(result, r#""Hello\n\'world\"!""#);
    }

    #[test]
    fn test_reset_escape_function() {
        let no_escape: super::EscapeFn = |input| input.to_string();
        let mut tera = Tera::default();
        tera.add_raw_template("foo", "{{ content }}").unwrap();
        tera.autoescape_on(vec!["foo"]);
        tera.set_escape_fn(no_escape);
        tera.reset_escape_fn();
        let mut context = Context::new();
        context.insert("content", &"Hello\n\'world\"!");
        let result = tera.render("foo", &context).unwrap();
        assert_eq!(result, "Hello\n&#x27;world&quot;!");
    }

    #[test]
    fn test_value_one_off_template() {
        let m = json!({
            "greeting": "Good morning"
        });
        let result =
            Tera::one_off("{{ greeting }} world", &Context::from_value(m).unwrap(), true).unwrap();

        assert_eq!(result, "Good morning world");
    }

    #[test]
    fn test_render_str_with_custom_function() {
        let mut tera = Tera::default();
        tera.register_function("echo", |args: &HashMap<_, JsonValue>| {
            Ok(args.get("greeting").map(JsonValue::to_owned).unwrap())
        });

        let result =
            tera.render_str("{{ echo(greeting='Hello') }} world", &Context::default()).unwrap();

        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_extend_no_overlap() {
        let mut my_tera = Tera::default();
        my_tera
            .add_raw_templates(vec![
                ("one", "{% block hey %}1{% endblock hey %}"),
                ("two", "{% block hey %}2{% endblock hey %}"),
                ("three", "{% block hey %}3{% endblock hey %}"),
            ])
            .unwrap();

        let mut framework_tera = Tera::default();
        framework_tera.add_raw_templates(vec![("four", "Framework X")]).unwrap();

        my_tera.extend(&framework_tera).unwrap();
        assert_eq!(my_tera.templates.len(), 4);
        let result = my_tera.render("four", &Context::default()).unwrap();
        assert_eq!(result, "Framework X");
    }

    #[test]
    fn test_extend_with_overlap() {
        let mut my_tera = Tera::default();
        my_tera
            .add_raw_templates(vec![
                ("one", "MINE"),
                ("two", "{% block hey %}2{% endblock hey %}"),
                ("three", "{% block hey %}3{% endblock hey %}"),
            ])
            .unwrap();

        let mut framework_tera = Tera::default();
        framework_tera
            .add_raw_templates(vec![("one", "FRAMEWORK"), ("four", "Framework X")])
            .unwrap();

        my_tera.extend(&framework_tera).unwrap();
        assert_eq!(my_tera.templates.len(), 4);
        let result = my_tera.render("one", &Context::default()).unwrap();
        assert_eq!(result, "MINE");
    }

    #[test]
    fn test_extend_new_filter() {
        let mut my_tera = Tera::default();
        let mut framework_tera = Tera::default();
        framework_tera.register_filter("hello", |_: &JsonValue, _: &HashMap<String, JsonValue>| {
            Ok(JsonValue::Number(10.into()))
        });
        my_tera.extend(&framework_tera).unwrap();
        assert!(my_tera.filters.contains_key("hello"));
    }

    #[test]
    fn test_extend_new_tester() {
        let mut my_tera = Tera::default();
        let mut framework_tera = Tera::default();
        framework_tera.register_tester("hello", |_: Option<&JsonValue>, _: &[JsonValue]| Ok(true));
        my_tera.extend(&framework_tera).unwrap();
        assert!(my_tera.testers.contains_key("hello"));
    }

    #[test]
    fn can_load_from_glob() {
        let tera = Tera::new("examples/basic/templates/**/*").unwrap();
        assert!(tera.get_template("base.html").is_ok());
    }

    #[test]
    fn can_load_from_glob_with_patterns() {
        let tera = Tera::new("examples/basic/templates/**/*.{html, xml}").unwrap();
        assert!(tera.get_template("base.html").is_ok());
    }

    #[test]
    fn full_reload_with_glob() {
        let mut tera = Tera::new("examples/basic/templates/**/*").unwrap();
        tera.full_reload().unwrap();

        assert!(tera.get_template("base.html").is_ok());
    }

    #[test]
    fn full_reload_with_glob_after_extending() {
        let mut tera = Tera::new("examples/basic/templates/**/*").unwrap();
        let mut framework_tera = Tera::default();
        framework_tera
            .add_raw_templates(vec![("one", "FRAMEWORK"), ("four", "Framework X")])
            .unwrap();
        tera.extend(&framework_tera).unwrap();
        tera.full_reload().unwrap();

        assert!(tera.get_template("base.html").is_ok());
        assert!(tera.get_template("one").is_ok());
    }

    #[should_panic]
    #[test]
    fn test_can_only_parse_templates() {
        let mut tera = Tera::parse("examples/basic/templates/**/*").unwrap();
        for tpl in tera.templates.values_mut() {
            tpl.name = format!("a-theme/templates/{}", tpl.name);
            if let Some(ref parent) = tpl.parent.clone() {
                tpl.parent = Some(format!("a-theme/templates/{}", parent));
            }
        }
        // Will panic here as we changed the parent and it won't be able
        // to build the inheritance chain in this case
        tera.build_inheritance_chains().unwrap();
    }

    // https://github.com/Keats/tera/issues/380
    #[test]
    fn glob_work_with_absolute_paths() {
        let tmp_dir = tempdir().expect("create temp dir");
        let cwd = tmp_dir.path().canonicalize().unwrap();
        File::create(cwd.join("hey.html")).expect("Failed to create a test file");
        File::create(cwd.join("ho.html")).expect("Failed to create a test file");
        let glob = cwd.join("*.html").into_os_string().into_string().unwrap();
        let tera = Tera::new(&glob).expect("Couldn't build Tera instance");
        assert_eq!(tera.templates.len(), 2);
    }

    #[test]
    fn glob_work_with_absolute_paths_and_double_star() {
        let tmp_dir = tempdir().expect("create temp dir");
        let cwd = tmp_dir.path().canonicalize().unwrap();
        File::create(cwd.join("hey.html")).expect("Failed to create a test file");
        File::create(cwd.join("ho.html")).expect("Failed to create a test file");
        let glob = cwd.join("**").join("*.html").into_os_string().into_string().unwrap();
        let tera = Tera::new(&glob).expect("Couldn't build Tera instance");
        assert_eq!(tera.templates.len(), 2);
    }

    // https://github.com/Keats/tera/issues/396
    #[test]
    fn issues_found_fuzzing_expressions_are_fixed() {
        let samples: Vec<(&str, Option<&str>)> = vec![
            // sample, expected result if it isn't an error
            ("{{0%0}}", None),
            ("{{W>W>vv}}{", None),
            ("{{22220222222022222220}}", None),
            ("{_{{W~1+11}}k{", None),
            ("{{n~n<n.11}}}", None),
            ("{{266314325266577770*4167}}7}}7", None),
            ("{{0~1~``~0~0~177777777777777777777~``~0~0~h}}", None),
        ];

        for (sample, expected_output) in samples {
            let res = Tera::one_off(sample, &Context::new(), true);
            if let Some(output) = expected_output {
                assert!(res.is_ok());
                assert_eq!(res.unwrap(), output);
            } else {
                assert!(res.is_err());
            }
        }
    }

    #[test]
    fn issues_found_fuzzing_conditions_are_fixed() {
        let samples: Vec<(&str, Option<&str>)> = vec![
            // (sample, expected result if it isn't an error)
            ("C~Q", None),
            ("s is V*0", None),
            ("x0x::N()", None),
            // this is an issue in pest itself: https://github.com/pest-parser/pest/issues/402
            //            ("_(p=__(p=[_(p=__(p=[_(p=[_(p=[_1", None),
        ];

        for (sample, expected_output) in samples {
            println!("{}, {:?}", sample, expected_output);
            let res = Tera::one_off(
                &format!("{{% if {} %}}true{{% endif %}}", sample),
                &Context::new(),
                true,
            );
            if let Some(output) = expected_output {
                assert!(res.is_ok());
                assert_eq!(res.unwrap(), output);
            } else {
                assert!(res.is_err());
            }
        }
    }
}
