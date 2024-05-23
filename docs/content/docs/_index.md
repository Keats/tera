+++
template = "docs.html"
insert_anchor_links = "right"
+++

# Getting started

To use Tera in your Rust projects, simply add it to your `Cargo.toml`:

```toml
tera = "1"
```

By default, Tera comes with some additional dependencies required for the `truncate`, `date`, `filesizeformat`, `slugify`, `urlencode` and `urlencode_strict` filters as
well as for the `now` function. You can disable them by setting the following in your `Cargo.toml`:

```toml
tera = { version = "1", default-features = false }
```


And add the following to your `lib.rs` or `main.rs` if you are not using Rust 2018 edition or later:

```rs
extern crate tera;
```

You can view everything Tera exports on the [API docs](https://docs.rs/tera).

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
let tera = match Tera::new("templates/**/*.html") {
    Ok(t) => t,
    Err(e) => {
        println!("Parsing error(s): {}", e);
        ::std::process::exit(1);
    }
};
```

Compiling templates is a step that is meant to only happen once: use something like [lazy_static](https://crates.io/crates/lazy_static)
to define a constant instance.

```rs
lazy_static! {
    pub static ref TEMPLATES: Tera = {
        let mut tera = match Tera::new("examples/basic/templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        tera.autoescape_on(vec![".html", ".sql"]);
        tera.register_filter("do_nothing", do_nothing_filter);
        tera
    };
}
```

You need two things to render a template: a name and a context.
If you are using globs, Tera will automatically remove the glob prefix from the template names. To use our example from before,
the template name for the file located at `templates/hello.html` will be `hello.html`.

The context can either be a data structure that implements the `Serialize` trait from `serde_json` or an instance of `tera::Context`:

```rs
use tera::Context;
// Using the tera Context struct
let mut context = Context::new();
context.insert("product", &product);
context.insert("vat_rate", &0.20);
tera.render("products/product.html", &context)?;

#[derive(Serialize)]
struct Product {
    name: String
}
// or a struct
tera.render("products/product.html", &Context::from_serialize(&product)?)?;
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

Tera does not perform contextual auto-escaping, e.g. by parsing the template to know whether to escape JS, CSS or HTML (see 
<https://rawgit.com/mikesamuel/sanitized-jquery-templates/trunk/safetemplate.html> for more details on that).

## Advanced usage

### Extending another instance
If you are using a framework or a library using Tera, chances are they provide their own Tera instance with some
built-in templates, filters, global functions or testers. Tera offers an `extend` method that will extend your own
instance with everything mentioned before:

```rs
let mut tera = Tera::new(&tpl_glob).chain_err(|| "Error parsing templates")?;
// ZOLA_TERA is an instance present in a library
tera.extend(&ZOLA_TERA)?;
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
Tera allows you to load templates not only from files but also from plain strings.

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
If some templates are related, for example one extending the other, you will need to use the `add_raw_templates` method
as Tera will error if it find inconsistencies such as extending a template that Tera doesn't know about.

### Render a one off template

Want to render a single template, for example one coming from a user? The `one_off` function is there for that.

```rs
// The last parameter is whether we want to autoescape the template or not.
// Should be true in 99% of the cases for HTML
let context = Context::new();
// add stuff to context
let result = Tera::one_off(user_tpl, context, true);
```


# Templates

## Introduction

### Tera Basics

A Tera template is just a text file where variables and expressions get replaced with values
when it is rendered. The syntax is based on Jinja2 and Django templates.

There are 3 kinds of delimiters and those cannot be changed:

- `{{` and `}}` for expressions
- `{%` and `%}` for statements
- `{#` and `#}` for comments

### Raw

Tera will consider all text inside the `raw` block as a string and won't try to
render what's inside. Useful if you have text that contains Tera delimiters.

```jinja2
{% raw %}
  Hello {{ name }}
{% endraw %}
```
would be rendered as `Hello {{ name }}`.

### Whitespace control

Tera comes with easy to use whitespace control: use `{%-` if you want to remove all whitespace
before a statement and `-%}` if you want to remove all whitespace after. This behavior also 
works with expressions, using `{{-` and `-}}`, and with comments, using `{#-` and `-#}`.

For example, let's look at the following template:

```jinja2
{% set my_var = 2 %}
{{ my_var }}
```

will have the following output:

```html

2
```

If we want to get rid of the empty line, we can write the following:

```jinja2
{% set my_var = 2 -%}
{{ my_var }}
```

### Comments
To comment out part of the template, wrap it in `{# #}`. Anything in between those tags
will not be rendered.

```jinja2
{# A comment #}
```

## Data structures

### Literals

Tera has a few literals that can be used:

- booleans: `true` (or `True`) and `false` (or `False`)
- integers
- floats
- strings: text delimited by `""`, `''` or ` `` `
- arrays: a comma-separated list of literals and/or idents surrounded by `[` and `]` (trailing comma allowed)

### Variables

Variables are defined by the context given when rendering a template. If you'd like to define your own variables, see the [Assignments](#assignments) section.

You can render a variable by using the `{{ name }}`.

Trying to access or render a variable that doesn't exist will result in an error.

A magical variable is available in every template if you want to print the current context: `__tera_context`.

#### Dot notation:
Construct and attributes can be accessed by using the dot (`.`) like `{{ product.name }}`.
Specific members of an array or tuple are accessed by using the `.i` notation, where i is a zero-based index. In dot notation variable can not be used after the dot (`.`).

#### Square bracket notation:
A more powerful alternative to (`.`) is to use square brackets (`[ ]`).
Variables can be rendered using the notation `{{product['name']}}` or `{{product["name"]}}`.

If the item is not in quotes it will be treated as a variable.
Assuming you have the following objects in your context `product = Product{ name: "Fred" }`
and `my_field = "name"`, calling `{{product[my_field]}}` will resolve to: `{{product.name}}`.

Only variables evaluating to string or integer number can be used as index: anything else will be
an error.

### Expressions

Tera allows expressions almost everywhere.

#### Math
You can do some basic math in Tera but it shouldn't be abused other than the occasional `+ 1` or similar.
Math operations are only allowed with numbers, using them on any other kind of values will result in an error.
You can use the following operators:

- `+`: adds 2 values together, `{{ 1 + 1 }}` will print `2`
- `-`: performs a subtraction, `{{ 2 - 1 }}` will print `1`
- `/`: performs a division, `{{ 10 / 2 }}` will print `5`
- `*`: performs a multiplication, `{{ 5 * 2 }}` will print `10`
- `%`: performs a modulo, `{{ 2 % 2 }}` will print `0`

The priority of operations is the following, from lowest to highest:

- `+` and `-`
- `*` and `/` and `%`


#### Comparisons

- `==`: checks whether the values are equal
- `!=`: checks whether the values are different
- `>=`: true if the left value is equal or greater to the right one
- `<=`: true if the right value is equal or greater to the left one
- `>`: true if the left value is greater than the right one
- `<`: true if the right value is greater than the left one

#### Logic

- `and`: true if the left and right operands are true
- `or`: true if the left or right operands are true
- `not`: negate an expression

#### Concatenation

You can concatenate several strings/numbers/idents using the `~` operator.

```jinja2
{{ "hello " ~ 'world' ~ `!` }}

{{ an_ident ~ " and a string" ~ another_ident }}

{{ an_ident ~ another_ident }}
```

An ident resolving to something other than a string or a number will raise an error.

#### `in` checking

You can check whether a left side is contained in a right side using the `in` operator.

```jinja2
{{ some_var in [1, 2, 3] }}

{{ 'index' in page.path }}

{{ an_ident not in  an_obj }}
```

Only literals/variables resulting in an array, a string and an object are supported in the right hand side: everything else
will raise an error. While in the left hand side only literals/variables resulting in a number, a string and a boolean are supported.


## Manipulating data

### Assignments
You can assign values to variables during the rendering.
Assignments in for loops and macros are scoped to their context but
assignments outside of those will be set in the global context. Furthermore, assignments
in for loop are valid until the end of the current iteration only.

```jinja2
{% set my_var = "hello" %}
{% set my_var = 1 + 4 %}
{% set my_var = some_var %}
{% set my_var = macros::some_macro() %}
{% set my_var = global_fn() %}
{% set my_var = [1, true, some_var | round] %}
```

If you want to assign a value in the global context while in a for loop, you can use `set_global`:

```jinja2
{% set_global my_var = "hello" %}
{% set_global my_var = 1 + 4 %}
{% set_global my_var = some_var %}
{% set_global my_var = macros::some_macro() %}
{% set_global my_var = global_fn() %}
{% set_global my_var = [1, true, some_var | round] %}
```
Outside of a for loop, `set_global` is exactly the same as `set`.

### Filters

You can modify variables using **filters**.
Filters are separated from the variable by a pipe symbol (`|`) and may have named arguments in parentheses.
Multiple filters can be chained: the output of one filter is applied to the next.

For example, `{{ name | lower | replace(from="doctor", to="Dr.") }}` will take a variable called name, make it lowercase and then replace instances of `doctor` by `Dr.`.
It is equivalent to `replace(lower(name), from="doctor", to="Dr.")` if we were to look at it as functions.

Calling filters on an incorrect type like trying to capitalize an array or using invalid types for arguments will result in an error.

Filters are functions with the `fn(Value, HashMap<String, Value>) -> Result<Value>` definition and custom ones can be added like so:

```rust
tera.register_filter("upper", string::upper);
```

While filters can be used in math operations, they will have the lowest priority and therefore might not do what you expect:


```css
{{ 1 + a | length }}
// is equal to
{{ (1 + a) | length } // this will probably error

// This will do what you wanted initially
{{ a | length + 1 }}
```
Tera has many [built-in filters](@/docs/_index.md#built-in-filters) that you can use.

#### Filter sections

Whole sections can also be processed by filters if they are encapsulated in `{% filter name %}` and `{% endfilter %}`
tags where `name` is the name of the filter:

```jinja2
{% filter upper %}
    Hello
{% endfilter %}
```

This example transforms the text `Hello` in all upper-case (`HELLO`).

Filter sections can also contain [`block` sections](@/docs/_index.md#inheritance) like this:
```jinja2
{% filter upper %}
  {% block content_to_be_upper_cased %}
    This will be upper-cased
  {% endblock content_to_be_upper_cased %} 
{% endfilter %}
```

### Tests

Tests can be used against an expression to check some condition on it and
are made in `if` blocks using the `is` keyword.
For example, you would write the following to test if an expression is odd:

```jinja2
{% if my_number is odd %}
 Odd
{% endif %}
```

Tests can also be negated:

```jinja2
{% if my_number is not odd %}
 Even
{% endif %}
```

Tests are functions with the `fn(Option<Value>, Vec<Value>) -> Result<bool>` definition and custom ones can be added like so:

```rust
tera.register_tester("odd", testers::odd);
```

Tera has many [built-in tests](@/docs/_index.md#built-in-tests) that you can use.

### Functions
Functions are Rust code that return a `Result<Value>` from the given params.

Quite often, functions will need to capture some external variables, such as a `url_for` global function needing
the list of URLs for example.

Here's an example on how to implement a very basic function:

```rust
fn make_url_for(urls: BTreeMap<String, String>) -> impl Function {
    Box::new(move |args| -> Result<Value> {
        match args.get("name") {
            Some(val) => match from_value::<String>(val.clone()) {
                Ok(v) =>  Ok(to_value(urls.get(&v).unwrap()).unwrap()),
                Err(_) => Err("oops".into()),
            },
            None => Err("oops".into()),
        }
    })
}
```
You then need to add it to Tera:

```rust
tera.register_function("url_for", make_url_for(urls));
```

And you can now call it from a template:

```jinja2
{{/* url_for(name="home") */}}
```

You can also implement the [trait](https://docs.rs/tera/1.5.0/tera/trait.Function.html) directly if you have more
complex requirements.

Currently functions can be called in two places in templates:

- variable block: `{{/* url_for(name="home") */}}`
- for loop container: `{% for i in range(end=5) %}`

Tera comes with some [built-in functions](@/docs/_index.md#built-in-functions).

## Control structures

### If

Conditionals are fully supported and are identical to the ones in Python.

```jinja2
{% if price < 10 or always_show %}
   Price is {{ price }}.
{% elif price > 1000 and not rich %}
   That's expensive!
{% else %}
    N/A
{% endif %}
```

Undefined variables are considered falsy. This means that you can test for the
presence of a variable in the current context by writing:

```jinja2
{% if my_var %}
    {{ my_var }}
{% else %}
    Sorry, my_var isn't defined.
{% endif %}
```
Every `if` statement has to end with an `endif` tag.

### For

Loop over items in a array:
```jinja2
{% for product in products %}
  {{loop.index}}. {{product.name}}
{% endfor %}
```

Or on characters of a string:

```jinja2
{% for letter in name %}
  {% if loop.index % 2 == 0%}
    <span style="color:red">{{ letter }}</span>
  {% else %}
    <span style="color:blue">{{ letter }}</span>
  {% endif %}
{% endfor %}
```

A few special variables are available inside for loops:

- `loop.index`: current iteration 1-indexed
- `loop.index0`: current iteration 0-indexed
- `loop.first`: whether this is the first iteration
- `loop.last`: whether this is the last iteration

Every `for` statement has to end with an `endfor` tag.

You can also loop on maps and structs using the following syntax:
```jinja2
{% for key, value in products %}
  {{key}}. {{value.name}}
{% endfor %}
```
`key` and `value` can be named however you want, they just need to be separated with a comma.

If you are iterating on an array, you can also apply filters to the container:

```jinja2
{% for product in products | reverse %}
  {{loop.index}}. {{product.name}}
{% endfor %}
```

You can also iterate on array literals:

```jinja2
{% for a in [1,2,3,] %}
  {{a}}
{% endfor %}
```

Lastly, you can set a default body to be rendered when the container is empty:


```jinja2
{% for product in products %}
  {{loop.index}}. {{product.name}}
{% else %}
  No products.  
{% endfor %}
```

#### Loop Controls

Within a loop, `break` and `continue` may be used to control iteration.

To stop iterating when `target_id` is reached:

```jinja2
{% for product in products %}
  {% if product.id == target_id %}{% break %}{% endif %}
  {{loop.index}}. {{product.name}}
{% endfor %}
```

To skip even-numbered items:
```jinja2
{% for product in products %}
  {% if loop.index is even %}{% continue %}{% endif %}
  {{loop.index}}. {{product.name}}
{% endfor %}
```

### Include

You can include a template to be rendered using the current context with the `include` tag.

```jinja
{% include "included.html" %}
```

The template path needs to be a static string. This is invalid:

```jinja
{% include "partials/" ~ name ~ ".html" %}
```

Tera doesn't offer passing a custom context to the `include` tag.
If you want to do that, use macros.

While you can `set` values in included templates, those values only exist while rendering
them: the template calling `include` doesn't see them.

You can mark an include with `ignore missing`, so that Tera will ignore the statement if the template to be included does not exist.

```jinja2
{% include "header.html" ignore missing %}
```

You can also provide a list of templates that are checked for existence before inclusion. The first template that exists will be included. If `ignore missing` is given, it will fall back to rendering nothing if none of the templates exist.

```jinja2
{% include ["custom/header.html", "header.html"] %}
{% include ["special_sidebar.html", "sidebar.html"] ignore missing %}
```

Note: `include` works similar to how it does in other engines like Jinja, with the exception that the current version of Tera doesn't allow inheritance within included files. Practically
speaking this means you have to choose between using `include`s or `extends` to organise your site, without mixing them. 

### Macros

Think of macros as functions or components that you can call and return some text.

They are defined as follows:

```jinja2
{% macro input(label, type="text") %}
    <label>
        {{ label }}
        <input type="{{type}}" />
    </label>
{% endmacro input %}
```
As shown in the example above, macro arguments can have a default [literal](@/docs/_index.md#literals) value.

If a macro is defined in a separate file, you need to import the file containing the macros:

```jinja2
{% import "macros.html" as macros %}
```
You can name that file namespace (`macros` in the example) anything you want.
A macro is called like this:

```jinja2
// namespace::macro_name(**kwargs)
{{ macros::input(label="Name", type="text") }}
```
Do note that macros, like filters, require keyword arguments.
Use the `self` namespace when calling a macro defined in the same file. Macros must be defined top-level (they cannot be nested in an if, for, etc.) and should only reference arguments, not template variables directly.


Macros can be called recursively but there is no limit to recursion so make sure your macro ends.

Here's an example of a recursive macro:

```jinja2
{% macro factorial(n) %}
  {% if n > 1 %}{{ n }} - {{ self::factorial(n=n-1) }}{% else %}1{% endif %}
{% endmacro factorial %}
```

A macro's body can contain all normal Tera syntax with the exception of macros definition, `block` and `extends`.


## Inheritance

Tera uses the same kind of inheritance as Jinja2 and Django templates:
you define a base template and extend it in child templates through blocks.
There can be multiple levels of inheritance (i.e. A extends B that extends C).

### Base template
A base template typically contains the basic document structure as well as
several `blocks` that can have content.

For example, here's a `base.html` almost copied from the Jinja2 documentation:

```jinja2
<!DOCTYPE html>
<html lang="en">
<head>
    {% block head %}
    <link rel="stylesheet" href="style.css" />
    <title>{% block title %}{% endblock title %} - My Webpage</title>
    {% endblock head %}
</head>
<body>
    <div id="content">{% block content %}{% endblock content %}</div>
    <div id="footer">
        {% block footer %}
        &copy; Copyright 2008 by <a href="http://domain.invalid/">you</a>.
        {% endblock footer %}
    </div>
</body>
</html>
```

This `base.html` template defines 4 `block` tags that child templates can override.
The `head` and `footer` block have some content already which will be rendered if they are not overridden.

### Child template
Again, straight from Jinja2 docs:

```jinja2
{% extends "base.html" %}
{% block title %}Index{% endblock title %}
{% block head %}
    {{/* super() */}}
    <style type="text/css">
        .important { color: #336699; }
    </style>
{% endblock head %}
{% block content %}
    <h1>Index</h1>
    <p class="important">
      Welcome to my awesome homepage.
    </p>
{% endblock content %}
```

To indicate inheritance, you have to use the `extends` tag as the first thing in the file followed by the name of the template you want
to extend.
The `{{/* super() */}}` variable call tells Tera to render the parent block there.

Nested blocks also work in Tera. Consider the following templates:

```jinja2
// grandparent
{% block hey %}hello{% endblock hey %}

// parent
{% extends "grandparent" %}
{% block hey %}hi and grandma says {{/* super() */}} {% block ending %}sincerely{% endblock ending %}{% endblock hey %}

// child
{% extends "parent" %}
{% block hey %}dad says {{/* super() */}}{% endblock hey %}
{% block ending %}{{/* super() */}} with love{% endblock ending %}
```
The block `ending` is nested in the `hey` block. Rendering the `child` template will do the following:

- Find the first base template: `grandparent`
- See `hey` block in it and check if it is in `child` and `parent` template
- It is in `child` so we render it, it contains a `super()` call so we render the `hey` block from `parent`,
which also contains a `super()` so we render the `hey` block of the `grandparent` template as well
- See `ending` block in `child`, render it and also render the `ending` block of `parent` as there is a `super()`

The end result of that rendering (not counting whitespace) will be: "dad says hi and grandma says hello sincerely with love".

This example explicitly terminates named blocks with `{% endblock hey %}`. It's not required to give the name of the block 
being terminated `{% endblock %}`, though it may add some clarity.

See the note in the [Include](@/docs/_index.md#include) section regarding mixing inheritance and includes.

## Built-ins

### Built-in filters

Tera has the following filters built-in:

#### lower
Converts a string to lowercase.

#### upper
Converts a string to uppercase.

#### wordcount
Returns the number of words in a string.

#### capitalize
Returns the string with all its characters lowercased apart from the first char which is uppercased.

#### replace
Takes 2 mandatory string named arguments: `from` and `to`. It will return a string with all instances of
the `from` string replaced with the `to` string.

Example: `{{ name | replace(from="Robert", to="Bob")}}`

#### addslashes
Adds slashes before quotes.

Example: `{{ value | addslashes }}`

If value is "I'm using Tera", the output will be "I\\'m using Tera".

#### slugify
Only available if the `builtins` feature is enabled.

Transforms a string into ASCII, lowercases it, trims it, converts spaces to hyphens and
removes all characters that are not numbers, lowercase letters or hyphens.

Example: `{{ value | slugify }}`

If value is "-Hello world! ", the output will be "hello-world".

#### title
Capitalizes each word inside a sentence.

Example: `{{ value | title }}`

If value is "foo  bar", the output will be "Foo  Bar".

#### trim
Removes leading and trailing whitespace if the variable is a string.

#### trim_start
Removes leading whitespace if the variable is a string.

#### trim_end
Removes trailing whitespace if the variable is a string.

#### trim_start_matches
Removes leading characters that match the given pattern if the variable is a string.

Example: `{{ value | trim_start_matches(pat="//") }}`

If value is "//a/b/c//", the output will be "a/b/c//".

#### trim_end_matches
Removes trailing characters that match the given pattern if the variable is a string.

Example: `{{ value | trim_end_matches(pat="//") }}`

If value is "//a/b/c//", the output will be "//a/b/c".

#### truncate
Only available if the `builtins` feature is enabled.

Truncates a string to the indicated length. If the string has a smaller length than
the `length` argument, the string is returned as is.

Example: `{{ value | truncate(length=10) }}`

By default, the filter will add an ellipsis at the end if the text was truncated. You can
change the string appended by setting the `end` argument.
For example, `{{ value | truncate(length=10, end="") }}` will not append anything.

#### linebreaksbr
Replaces line breaks (`\n` or `\r\n`) with HTML line breaks (`<br>`).

Example: `{{ value | linebreaksbr }}`

If value is "Hello\r\nworld\n", the output will be "Hello&lt;br&gt;world&lt;br&gt;".

Note that if the template you are using it in is automatically escaped, you will
need to call the `safe` filter after `linebreaksbr`.

#### spaceless
Remove space (` `) and line breaks (`\n` or `\r\n`) between HTML tags

Example: `{{ value | spaceless }}`

If the value is "&lt;p&gt;\n&lt;a&gt; &lt;/a&gt;\r\n &lt;/p&gt;", the output will be "&lt;p&gt;&lt;a&gt;&lt;/a&gt;&lt;/p&gt;".

Note that only whitespace between successive opening tags and successive closing tags is removed.

Also note that if the template you are using it in is automatically escaped, you will
need to call the `safe` filter after `spaceless`.

#### indent
Indents a string by injecting a prefix at the start of each line.  The `prefix` argument (default 4 spaces) specifies the prefix to insert per line.  If the `first` argument (default false) is set true spaces are inserted for the first line.  If the `blank` argument (default false) is set true spaces are inserted for blank/whitespace lines.

#### striptags
Tries to remove HTML tags from input. Does not guarantee well formed output if input is not valid HTML.

Example: `{{ value | striptags }}`

If value is "&lt;b&gt;Joel&lt;/b&gt;", the output will be "Joel".

Note that if the template you are using it in is automatically escaped, you will need to call the `safe` filter
after `striptags`.

#### first
Returns the first element of an array.
If the array is empty, returns empty string.

#### last
Returns the last element of an array.
If the array is empty, returns empty string.

#### nth
Returns the nth element of an array.§
If the array is empty, returns empty string.
It takes a required `n` argument, corresponding to the 0-based index you want to get.

Example: `{{ value | nth(n=2) }}`

#### join
Joins an array with a string.

Example: `{{ value | join(sep=" // ") }}`

If value is the array `['a', 'b', 'c']`, the output will be the string "a // b // c".

#### length
Returns the length of an array, an object, or a string.

#### reverse
Returns a reversed string or array.

#### sort
Sorts an array into ascending order.

The values in the array must be a sortable type:
- numbers are sorted by their numerical value.
- strings are sorted in alphabetical order.
- arrays are sorted by their length.
- bools are sorted as if false=0 and true=1

If you need to sort a list of structs or tuples, use the `attribute`
argument to specify which field to sort by.

Example:

Given `people` is an array of Person

```rust
struct Name(String, String);

struct Person {
    name: Name,
    age: u32,
}
```

The `attribute` argument can be used to sort by last name:

```jinja2
{{ people | sort(attribute="name.1") }}
```

or by age:

```jinja2
{{ people | sort(attribute="age") }}
```

#### unique
Removes duplicate items from an array.  The `attribute` argument can be used to select items based on the values of an inner attribute.  For strings, the `case_sensitive` argument (default is false) can be used to control the comparison.

Example:

Given `people` is an array of Person

```rust
struct Name(String, String);

struct Person {
    name: Name,
    age: u32,
}
```

The `attribute` argument can be used to select one Person for each age:

```jinja2
{{ people | unique(attribute="age") }}
```

or by last name:

```jinja2
{{ people | unique(attribute="name.1", case_sensitive="true") }}
```

#### slice
Slices an array by the given `start` and `end` parameter. Both parameters are
optional and omitting them will return the same array.
Use the `start` argument to define where to start (inclusive, default to `0`)
and `end` argument to define where to stop (exclusive, default to the length of the array).
`start` and `end` are 0-indexed.

```jinja2
{% for i in my_arr | slice(end=5) %}
{% for i in my_arr | slice(start=1) %}
{% for i in my_arr | slice(start=1, end=5) %}
```

You can also use negative index values to refer the array from the last element. -1 refers to the
last index, -2 refers to the second last index and so on.

For example, let's look at the following template:

```jinja2
{% for i in my_arr | slice(end=-2) %}
```

will produce the follow output for `my_array = [1, 2, 3, 4, 5]`: `[1, 2, 3]`


#### group_by
Groups an array using the required `attribute` argument. The filter takes an array and returns
a map where the keys are the values of the `attribute` stringified and the values are all elements of
the initial array having that `attribute`. Values with missing `attribute` or where `attribute` is null
will be discarded.

Example:

Given `posts` is an array of Post

```rust
struct Author {
    name: String,
};

struct Post {
    content: String,
    year: u32,
    author: Author,
}
```

The `attribute` argument can be used to group posts by year:

```jinja2
{{ posts | group_by(attribute="year") }}
```

or by author name:

```jinja2
{% for name, author_posts in posts | group_by(attribute="author.name") %}
    {{ name }}
    {% for post in author_posts %}
        {{ post.year }}: {{ post.content }}
    {% endfor %}
{% endfor %}
```

Manipulating the hashmap produced by `group_by` in an arbitrary order requires additional steps to extract the keys into a separate array.

Example:

```jinja2
{% set map = section.pages | group_by(attribute="year") %}
{% set_global years = [] %}
{% for year, ignored in map %}
    {% set_global years = years | concat(with=year) %}
{% endfor %}
{% for year in years | reverse %}
    {% set posts = map[year] %}
{% endfor %}
```

#### filter

Filters the array values, returning only the values where the `attribute` is equal to the `value`.
Values with missing `attribute` or where `attribute` is null will be discarded.

`attribute` is mandatory.


Example:

Given `posts` is an array of Post

```rust
struct Author {
    name: String,
};

struct Post {
    content: String,
    year: u32,
    author: Author,
    draft: bool,
}
```

The `attribute` argument can be used to filter posts by draft value:

```jinja2
{{ posts | filter(attribute="draft", value=true) }}
```

or by author name:

```jinja2
{{ posts | filter(attribute="author.name", value="Vincent") }}
```

If `value` is not passed, it will drop any elements where the attribute is `null`.

#### map

Retrieves an attribute from each object in an array.  The `attribute` argument is mandatory and specifies what to extract.

Example:

Given `people` is an array of Person

```rust
struct Name(String, String);

struct Person {
    name: Name,
    age: u32,
}
```

The `attribute` argument is used to retrieve their ages.

```jinja2
{{ people | map(attribute="age") }}
```

#### concat
Appends values to an array.

```jinja2
{{ posts | concat(with=drafts) }}
```

The filter takes an array and returns a new array with the value(s) from the `with` parameter
added. If the `with` parameter is an array, all of its values will be appended one by one to the new array and
not as an array.

This filter can also be used to append a single value to an array if the value passed to `with` is not an array:

```jinja2
{% set pages_id = pages_id | concat(with=id) %}
```

The `with` attribute is mandatory.

#### urlencode
Only available if the `builtins` feature is enabled.

Percent-encodes all the characters in a string which are not included in
unreserved chars (according to [RFC3986](https://tools.ietf.org/html/rfc3986)) with the exception of forward
slash (`/`).

Example: `{{ value | urlencode }}`

If value is `/foo?a=b&c=d`, the output will be `/foo%3Fa%3Db%26c%3Dd`. `/` is not escaped.

#### urlencode_strict
Only available if the `builtins` feature is enabled.

Similar to `urlencode` filter but encodes all non-alphanumeric characters in a string including forward slashes (`/`).

Example: `{{ value | urlencode_strict }}`

If value is `/foo?a=b&c=d`, the output will be `%2Ffoo%3Fa%3Db%26c%3Dd`. `/` is
also encoded.

#### abs
Returns the absolute value

Example: `{{ negative_number | abs }}`

If negative_number is -1, the output will be 1. If num_messages is -2.0 the output will be 2.

#### pluralize
Returns a plural suffix if the value is not equal to ±1, or a singular suffix otherwise. The plural suffix defaults to `s` and the
singular suffix defaults to the empty string (i.e. nothing).

Example: `You have {{ num_messages }} message{{ num_messages | pluralize }}`

If num_messages is 1, the output will be You have 1 message. If num_messages is 2 the output will be You have 2 messages. You can
also customize the singular and plural suffixes with the `singular` and `plural` arguments to the filter:

Example: `{{ num_categories }} categor{{ num_categories | pluralize(singular="y", plural="ies") }}`

#### round
Returns a number rounded following the method given. Default method is `common` which will round to the nearest integer.
`ceil` and `floor` are available as alternative methods.
Another optional argument, `precision`, is available to select the precision of the rounding. It defaults to `0`, which will
round to the nearest integer for the given method.

Example: `{{ num | round }} {{ num | round(method="ceil", precision=2) }}`

#### filesizeformat
Only available if the `builtins` feature is enabled.

Returns a human-readable file size (i.e. '110 MB') from an integer.

Example: `{{ num | filesizeformat }}`

#### date
Only available if the `builtins` feature is enabled.

Parses a timestamp into a date(time) string. Defaults to `YYYY-MM-DD` format.
Time formatting syntax is inspired from strftime and a full reference is available
on [chrono docs](https://docs.rs/chrono/0.4/chrono/format/strftime/index.html).

Example: `{{ ts | date }} {{ ts | date(format="%Y-%m-%d %H:%M") }}`

If you are using ISO 8601 date strings or a UTC timestamp, you can optionally supply a timezone for the date to be rendered in.

Example:

```
{{ "2019-09-19T13:18:48.731Z" | date(timezone="America/New_York") }}

{{ "2019-09-19T13:18:48.731Z" | date(format="%Y-%m-%d %H:%M", timezone="Asia/Shanghai") }}

{{ 1648252203 | date(timezone="Europe/Berlin") }}
```

Locale can be specified (excepted when the input is a timestamp without timezone argument), default being POSIX. (only available if the `date-locale` feature is enabled)

Example: `{{ 1648252203 | date(format="%A %-d %B", timezone="Europe/Paris", locale="fr_FR") }}`

#### escape
Escapes a string's HTML. Specifically, it makes these replacements:

- `&` is converted to `&amp;`
- `<` is converted to `&lt;`
- `>` is converted to `&gt;`
- `"` (double quote) is converted to `&quot;`
- `'` (single quote) is converted to `&#x27;`
- `/` is converted to `&#x2F;`

#### escape_xml
Escapes XML special characters. Specifically, it makes these replacements:

- `&` is converted to `&amp;`
- `<` is converted to `&lt;`
- `>` is converted to `&gt;`
- `"` (double quote) is converted to `&quot;`
- `'` (single quote) is converted to `&apos;`

#### safe
Marks a variable as safe: HTML will not be escaped anymore.
`safe` only works if it is the last filter of the expression:

- `{{ content | replace(from="Robert", to="Bob") | safe }}` will not be escaped
- `{{ content | safe | replace(from="Robert", to="Bob") }}` will be escaped

#### get
Accesses a value from an object when the key is not a Tera identifier.
Example: `{{ sections | get(key="posts/content") }}`

The `get` filter also has a `default` parameter which can be used to provide a return value when the `key` parameter is missing from the set being filtered.
Example: `{{ sections | get(key="posts/content", default="default") }}`

#### split
Splits a string into an array of strings, separated by a pattern given.
Example: `{{ path | split(pat="/") }}`

#### int
Converts a value into an integer.  The `default` argument can be used to specify the value to return on error, and the `base` argument can be used to specify how to interpret the number.  Bases of 2, 8, and 16 understand the prefix 0b, 0o, 0x, respectively.

#### float
Converts a value into a float.  The `default` argument can be used to specify the value to return on error.

#### json_encode
Transforms any value into a JSON representation. This filter is better used together with `safe` or when automatic escape is disabled.

Example: `{{ value | json_encode() | safe }}`

It accepts a parameter `pretty` (boolean) to print a formatted JSON instead of a one-liner.

Example: `{{ value | json_encode(pretty=true) | safe }}`

#### as_str
Returns a string representation of the given value.

Example: `{{ value | as_str }}`

#### default
Returns the default value given only if the variable evaluated is not present in the context
and is therefore meant to be at the beginning of a filter chain if there are several filters.

Example: `{{ value | default(value=1) }}`

This is in most cases a shortcut for:

```jinja2
{% if value %}{{ value }}{% else %}1{% endif %}
```

However, only the existence of the value in the context is checked. With a value that `if` would
evaluate to false (such as an empty string, or the number 0), the `default` filter will not attempt
replace it with the alternate value provided. For example, the following will produce
"I would like to read more !":

```jinja2
I would like to read more {{ "" | default (value="Louise Michel") }}!
```

If you intend to use the default filter to deal with optional values, you should make sure those values
aren't set! Otherwise, use a full `if` block. This is especially relevant for dealing with optional arguments
passed to a macro.

### Built-in tests

Here are the currently built-in tests:

#### defined
Returns true if the given variable is defined.

#### undefined
Returns true if the given variable is undefined.

#### odd
Returns true if the given variable is an odd number.

#### even
Returns true if the given variable is an even number.

#### string
Returns true if the given variable is a string.

#### number
Returns true if the given variable is a number.

#### divisibleby
Returns true if the given expression is divisible by the arg given.

Example:
```jinja2
{% if rating is divisibleby(2) %}
    Divisible
{% endif %}
```

#### iterable
Returns true if the given variable can be iterated over in Tera (i.e. is an array/tuple or an object).

#### object
Returns true if the given variable is an object (i.e. can be iterated over key, value).

#### starting\_with
Returns true if the given variable is a string and starts with the arg given.

Example:
```jinja2
{% if path is starting_with("x/") %}
    In section x
{% endif %}
```

#### ending\_with
Returns true if the given variable is a string and ends with the arg given.

#### containing
Returns true if the given variable contains the arg given.

The test works on:

- strings: is the arg a substring?
- arrays: is the arg given one of the members of the array?
- maps: is the arg given a key of the map?

Example:
```jinja2
{% if username is containing("xXx") %}
    Bad
{% endif %}
```

#### matching
Returns true if the given variable is a string and matches the regex in the argument.

Example:
```jinja2
{% if name is matching("^[Qq]ueen") %}
    Her Royal Highness, {{ name }}
{% elif name is matching("^[Kk]ing") %}
    His Royal Highness, {{ name }}
{% else %}
    {{ name }}
{% endif %}
```

A comprehensive syntax description can be found in the [regex crate documentation](https://docs.rs/regex/).

### Built-in functions
Tera comes with some built-in global functions.

#### range

Returns an array of integers created using the arguments given.
There are 3 arguments, all integers:

- `end`: stop before `end`, mandatory
- `start`: where to start from, defaults to `0`
- `step_by`: with what number do we increment, defaults to `1`


#### now
Only available if the `builtins` feature is enabled.

Returns the local datetime as string or the timestamp as integer if requested.

There are 2 arguments, both booleans:

- `timestamp`: whether to return the timestamp instead of the datetime
- `utc`: whether to return the UTC datetime instead of the local one

Formatting is not built-in the global function but you can use the `date` filter like so `now() | date(format="%Y")` if you
wanted to get the current year.

#### throw
The template rendering will error with the given message when encountered.

There is only one string argument:

- `message`: the message to display as the error

#### get_random
Only available if the `builtins` feature is enabled.

Returns a random integer in the given range. There are 2 arguments, both integers:

- `start`: defaults to 0 if not present
- `end`: required

`start` is inclusive (i.e. can be returned) and `end` is exclusive.

#### get_env
Returns the environment variable value for the name given. It will error if the environment variable is not found
but the call can also take a default value instead.

- `name`: the name of the environment variable to look for, required
- `default`: a default value in case the environment variable is not found

If the environment variable is found, it will always be a string while your default could be of any type.
