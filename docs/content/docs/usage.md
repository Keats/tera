+++
weight = 5
+++

# Usage

The primary method of using Tera is to load and parse all the templates in a given glob.

Let's take the following directory as example.

```sh
templates/
  hello.html
  index.html
  products/
    product.html
    price.html
```

Assuming the Rust file is at the same level as the `templates` folder, we can get a Tera instance that way:

```rs
use tera::Tera;

// Use globbing
let tera = compile_templates!("templates/**/*");
```

The `compile_templates!` macro will parse all files found in the glob and, if errors are encountered, exit the process after
printing the errors.
If you don't want to exit the process on errors, you can call the `Tera::new` method and handle errors directly.

Compiling templates is a step that is meant to only happen once: use something like [lazy_static](https://crates.io/crates/lazy_static)
to define a constant instance.

```rs
lazy_static! {
    pub static ref TERA: Tera = {
        let mut tera = compile_templates!("templates/**/*");
        // and we can add more things to our instance if we want to
        tera.autoescape_on(vec!["html", ".sql"]);
        tera.register_filter("do_nothing", do_nothing_filter);
        tera
    };
}
```

You need two things to render a template: a name and a context.
If you are using globs, Tera will automatically remove the glob prefix from the template names. To use our example from before,
the template name for the file located at `templates/hello.html` will be `hello.html`.

The context can either a be data structure that implements the `Serialize` trait from `serde_json` or an instance of `tera::Context`:

```rs
use tera::Context;
// Using the tera Context struct
let mut context = Context::new();
context.add("product", &product);
context.add("vat_rate", &0.20);
tera.render("products/product.html", &context)?;

#[derive(Serialize)]
struct Product {
    name: String
}
// or a struct
tera.render("products/product.html", &product)?;
```

## Auto-escaping
By default, Tera will auto-escape all content in files ending with `".html"`, `".htm"` and `".xml"`.
Escaping follows the recommendations from [OWASP](https://www.owasp.org/index.php/XSS_(Cross_Site_Scripting)_Prevention_Cheat_Sheet).

You can override that or completely disable auto-escaping by calling the `autoescape_on` method:

```rs
// escape only files ending with `.php.html`
tera.autoescape_on(vec![".php.html"]);
// disable autoescaping completely
tera.autoescape_on(vec![]);
```

## Advanced usage

### Extending another instance
If you are using a framework or a library using Tera, chances are they provide their own Tera instance with some
built-in templates, filters, global functions or testers. Tera offers a `extend` method that will extend your own
instance with everything mentioned before:

```rs
let mut tera = Tera::new(&tpl_glob).chain_err(|| "Error parsing templates")?;
// GUTENBERG_TERA is an instance present in a library
tera.extend(&GUTENBERG_TERA)?;
```
If anything - templates, filters, etc - with the same name exists in both instances, Tera will only keep yours.

### Reloading
If you are watching a directory and want to reload templates on change (editing/adding/removing a template), Tera gives
the `full_reload` method:

```rs
tera.full_reload()?;
```

Note that reloading is only available if you are loading templates with a glob.

### Loading templates from strings
Tera allows you load templates not only from files but also from plain strings.

```rs
// one template only
let mut tera = Tera::default();
tera.add_raw_template("hello.html", "the body")?;

// many templates
let mut tera = Tera::default();
tera.add_raw_templates(vec![
    ("grandparent", "{% block hey %}hello{% endblock hey %}"),
    ("parent", "{% extends \"grandparent\" %}{% block hey %}Parent{% endblock hey %}"),
])?;
```
If some templates are related, for example one extending the other, you will need to the `add_raw_templates` method
as Tera will error if it find inconsistencies such as extending a template that Tera doesn't know about.

### Render a one off template

Want to render a single template, for example one coming from a user? The `one_off` function is there for that.

```rs
// The last parameter is whether we want to autoescape the template or not.
// Should be true in 99% of the cases for HTML
let context = Context::new();
// add stuff to context
let result = Tera::one_off(user_tpl, &context, true);
```
