+++
insert_anchor_links = "right"
+++

{% raw %}
## Getting started

If you are coming from Tera v1, see the [migration guide](https://github.com/Keats/tera2/blob/master/MIGRATION.md).

To use Tera in your Rust projects, simply add it to your `Cargo.toml`:

```toml
tera = "2"
```

By default, Tera only pulls one dependency: `serde`.

A few features requiring more dependencies are available:

- `fast`: speed up template rendering (you can also select only some of the features from that feature group)
- `glob_fs`: allows loading template on the filesystem using a glob
- `unicode`: if you want Tera to work with graphemes clusters rather than utf-8 characters when iterating on strings
- `preserve_order`: keep order of insertion for values

There is also an additional crate, `tera-contrib`, which contains filters/functions/tests that require third party
dependencies. See its [README](https://github.com/Keats/tera2/tree/master/tera-contrib) for the list of features and
what's available.

## API

### Adding templates

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

```rust
use tera::Tera;

let mut tera = Tera::default();

// with the `glob_fs` feature enabled
tera.load_from_glob("templates/**/*.html")?;

// or by hand
tera.add_raw_templates(vec![
    ("hello.html", include_str!("templates/hello.html")),
    ("index.html", include_str!("templates/index.html")),
    ("products/product.html", include_str!("templates/products/product.html")),
    ("products/price.html", include_str!("templates/products/price.html")),
])?;
```

If you need to register custom functions/filters/tests, make sure to do so _before_ adding the templates as otherwise
you will get an error since they can't be found.

### Rendering a template

You need two things to render a template: a name and a context.
If you are using globs, Tera will automatically remove the glob prefix from the template names. To use our example from before,
the template name for the file located at `templates/hello.html` will be `hello.html`.

The context has to be an instance of `tera::Context`:

```rust
use tera::Context;

let mut context = Context::new();
context.insert("age", &24);
context.insert("product", &product);
// if you already have a Value, you can do the following to avoid serialization
context.insert_value("product2", product_value);

// There is also a `context!` macro which will always serialize
let context = context! {
    age => &24,
    product => &product,
};

tera.render("hello.html", &context)?;
```

You can also set a global context that will be automatically added to every template render context:

```rust
tera.global_context().insert("name", "John Doe");
```

### Auto-escaping
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

### Custom filters, tests and functions

Filters/Tests/Functions have a very similar signature, and they all take at least the same 2 arguments:

- [Kwargs](TODO: link to docs.rs): to extract the keyword arguments, in the right type handling errors automatically
- [State](TODO: link to docs.rs): to access the current context

Filters and tests take the value it's running on as the first parameter: this parameter can be a classic `Value` or
any value we can convert to. For example, the simplest filter is the following:

```rust
pub fn upper(val: &str, _: Kwargs, _: &State) -> String {
    val.to_uppercase()
}
```

This takes a value that needs to be string, anything else will automatically get an error. This also returns a string
and Tera will automatically convert it to the right type.

While they can be defined as plain functions, they each have a trait so you can implement it for a struct for example.
Here's an example from `tera-contrib`:

```rust
#[derive(Debug, Default)]
pub struct Matching {
  cache: RwLock<HashMap<String, Regex>>,
}

impl Test<&str, TeraResult<bool>> for Matching {
  fn call(&self, val: &str, kwargs: Kwargs, _: &State) -> TeraResult<bool> {
    let pat = kwargs.must_get::<&str>("pat")?;
    let regex = get_or_create_regex(&self.cache, pat)?;
    Ok(regex.is_match(val))
  }
}
```

Notice how to get arguments from the `Kwargs` instance. Missing arguments and type mismatch are handled automatically
for you and will report the error properly without you having to do any work.
See the docs.rs documentation for more details.

## Template

### Synopsis

A Tera template is just a text file where variables and expressions get replaced with values
when it is rendered. The syntax is based on Jinja2 and Django templates.

There are 3 kinds of delimiters and those can be changed via `Tera::set_delimiters`:

- `{{` and `}}` for expressions
- `{%` and `%}` for statements
- `{#` and `#}` for comments

### Literals

Tera has a few literals that can be used:

- booleans: `true` (or `True`) and `false` (or `False`)
- integers
- floats
- strings: text delimited by `""`, `''` or ` `` `
- arrays: a comma-separated list of literals and/or idents surrounded by `[` and `]` (trailing comma allowed)
- maps: a key/value literal inside `{..}` like `{"a": 1, "b": 2}` (trailing comma allowed)
- `none` (or `None`)

`none` is different from undefined: the value is `none` but it exists. Undefined means the value doesn't exist.

### Variables

Variables are defined by the context given when rendering a template. If you'd like to define your own variables, see the [Assignments](#assignments) section.

You can render a variable by using the `{{ name }}`.

Trying to access or render a variable that doesn't exist will result in an error.

A magical variable is available in every template if you want to print the current context: `__tera_context`.

#### Dot notation:
Constructs and attributes can be accessed by using the dot (`.`) like `{{ product.name }}`.
Specific members of an array or tuple are accessed by using the `.i` notation, where i is a zero-based index.
In dot notation variable can not be used after the dot (`.`).

#### Square bracket notation:
A more powerful alternative to (`.`) is to use square brackets (`[ ]`).
Variables can be rendered using the notation `{{product['name']}}` or `{{product["name"]}}`.

If the item is not in quotes it will be treated as a variable.
Assuming you have the following objects in your context `product = Product{ name: "Fred" }`
and `my_field = "name"`, calling `{{product[my_field]}}` will resolve to: `{{product.name}}`.

Only variables evaluating to string or integer number can be used as index: anything else will be
an error.

#### Optional chaining

Since we only allow one level of undefined-ness and we don't want to write a default filter for each access, we can use
optional chaining like in JS: `{{ a?.b?.c or "should print" }}`. This will try to load `a.b.c` but short-circuiting if any
value is null or undefined.

The syntax for optional arrays access is different from JS: `{{ a?['b']?.c or "should print" }}` is different from JS where
you would do `a?.['b']`.

### Expressions

Tera allows expressions everywhere.

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

#### String concatenation

You can concatenate several strings/numbers/idents using the `~` operator.

```jinja
{{ "hello " ~ 'world' ~ `!` }}

{{ an_ident ~ " and a string" ~ another_ident }}

{{ an_ident ~ another_ident }}
```

The output of a `~` operator will always be a string.

#### `in` checking

You can check whether a left side is contained in a right side using the `in` operator.

```jinja
{{ some_var in [1, 2, 3] }}

{{ 'index' in page.path }}

{{ an_ident not in  an_obj }}
```

Only literals/variables resulting in an array, a string and a map are supported on the right hand side: everything else
will raise an error. While on the left hand side only literals/variables resulting in an integer, a string and a boolean are supported.

#### Spread

If you've used JS, you will be familiar with that syntax:

```jinja
{% set m = {...base, "d": 4} %}
```

This creates a new variable `m` with all the fields from `base` with the `d` value updated to `4`.

Spread also work for arrays:

```jinja
{{ [0, ...numbers, 99] }}
```
#### Slicing

You can use slicing on your arrays, similar to Python slicing:

```jinja
{{ numbers[0] }}
{{ numbers[-1] }}
{{ numbers[:-1] }}
{{ numbers[:2] }}
{{ numbers[1:2] }}
{{ numbers[0:2:2] }}
{{ numbers[::-1] }}
{{ product.name[-1] }}
{{ product.name[::-1] }}
{{ product.name[1:] }}
{{ product.name[:-1] }}
```

`-1` means the last item of the array and the syntax is `[start:stop:step]`, like Python.

#### Ternary

You can do `{{ "majeur" if age >= 18 else "mineur" }}`. Both `if` and `else` are required.

### Filters

You can modify variables using **filters**.
Filters are separated from the variable by a pipe symbol (`|`) and may have named arguments in parentheses.
Multiple filters can be chained: the output of one filter is applied to the next.

For example, `{{ name | lower | replace(from="doctor", to="Dr.") }}` will take a variable called name, make it lowercase and then replace instances of `doctor` by `Dr.`.
It is equivalent to `replace(lower(name), from="doctor", to="Dr.")` if we were to look at it as functions.

Filters can be used inline like shown or as a filter section:

```jinja
{% filter my_filter(param="value") %}
some content {{ var }}
{% endfilter %}
```

Tera has many [built-in filters](#built-in-filters) that you can use.

### Tests

Tests can be used against an expression to check some condition on it and
are made in `if` blocks using the `is` keyword.
For example, you would write the following to test if an expression is odd:

```jinja
{% if my_number is odd %}
 Odd
{% endif %}
```

Tests can also be negated:

```jinja
{% if my_number is not odd %}
 Even
{% endif %}
```

Tera has many [built-in tests](#built-in-tests) that you can use.

### Functions

Functions are Rust code that return a `Result<Value>` from the given params.

They are called like so `{{ get_page(path="hello.md") }}`.

Tera comes with a couple of [built-in functions](#built-in-functions).

### Comments
To comment out part of the template, wrap it in `{# #}`. Anything in between those tags
will not be rendered.

```jinja
{# A comment #}
```

### Raw

Tera will consider all text inside the `raw` block as a string and won't try to
render what's inside. Useful if you have text that contains Tera delimiters.

```jinja
{% raw %}
  Hello {{ name }}
{% endraw %}{{ "{% endraw %}" }}{% raw %}
```
would be rendered as `Hello {{ name }}`.

### Whitespace control

Tera comes with easy to use whitespace control: use `{%-` if you want to remove all whitespace
before a statement and `-%}` if you want to remove all whitespace after. This behavior also
works with expressions, using `{{-` and `-}}`, and with comments, using `{#-` and `-#}`.

For example, let's look at the following template:

```jinja
{% set my_var = 2 %}
{{ my_var }}
```

will have the following output:

```html

2
```

If we want to get rid of the empty line, we can write the following:

```jinja
{% set my_var = 2 -%}
{{ my_var }}
```

### Control structures

#### If

Conditionals are fully supported and are identical to the ones in Python.

```jinja
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

```jinja
{% if my_var %}
    {{ my_var }}
{% else %}
    Sorry, my_var isn't defined.
{% endif %}
```
Every `if` statement has to end with an `endif` tag.


#### For

Loop over items in a array:
```jinja
{% for product in products %}
  {{loop.index}}. {{product.name}}
{% endfor %}
```

Or on characters of a string:

```jinja
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
- `loop.length`: total number of items in the iterable

Every `for` statement has to end with an `endfor` tag.

You can also loop on maps and structs using the following syntax:
```jinja
{% for key, value in products %}
  {{key}}. {{value.name}}
{% endfor %}
```
`key` and `value` can be named however you want, they just need to be separated with a comma.

If you are iterating on an array, you can also apply filters to the container:

```jinja
{% for product in products | reverse %}
  {{loop.index}}. {{product.name}}
{% endfor %}
```

You can also iterate on array literals:

```jinja
{% for a in [1,2,3,] %}
  {{a}}
{% endfor %}
```

You can also set a default body to be rendered when the container is empty:

```jinja
{% for product in products %}
  {{loop.index}}. {{product.name}}
{% else %}
  No products.
{% endfor %}
```

Within a loop, `break` and `continue` may be used to control iteration.

To stop iterating when `target_id` is reached:

```jinja
{% for product in products %}
  {% if product.id == target_id %}{% break %}{% endif %}
  {{loop.index}}. {{product.name}}
{% endfor %}
```

To skip even-numbered items:
```jinja
{% for product in products %}
  {% if loop.index is even %}{% continue %}{% endif %}
  {{loop.index}}. {{product.name}}
{% endfor %}
```

### Assignments

You can assign values to variables during the rendering.
Assignments in for loops and components are scoped to their context but
assignments outside of those will be set in the global context. Furthermore, assignments
in for loop are valid until the end of the current iteration only.

```jinja
{% set my_var = "hello" %}
{% set my_var = 1 + 4 %}
{% set my_var = some_var %}
{% set my_var = <my.component /> %}
{% set my_var = global_fn() %}
{% set my_var = [1, true, some_var | round] %}
```

If you want to assign a value in the global context while in a for loop, you can use `set_global`:

```jinja
{% set_global my_var = "hello" %}
{% set_global my_var = 1 + 4 %}
{% set_global my_var = some_var %}
{% set_global my_var = <my.component /> %}
{% set_global my_var = global_fn() %}
{% set_global my_var = [1, true, some_var | round] %}
```
Outside of a for loop, `set_global` is exactly the same as `set`.


You can use `set` with a body and apply filters to it:

```jinja
{% set hero | upper | trans(lang="fr") %}
Hello {{ world }}
{% endset %}
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
If you want to do that, use components.

While you can `set` values in included templates, those values only exist while rendering
them: the template calling `include` doesn't see them.

### Inheritance

Tera uses the same kind of inheritance as Jinja2 and Django templates:
you define a base template and extend it in child templates through blocks.
There can be multiple levels of inheritance (i.e. A extends B that extends C).

#### Base template
A base template typically contains the basic document structure as well as
several `blocks` that can have content.

For example, here's a `base.html` almost copied from the Jinja2 documentation:

```jinja
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

#### Child template
Again, straight from Jinja2 docs:

```jinja
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
The `{{ super() }}` variable call tells Tera to render the parent block there.

Nested blocks also work in Tera. Consider the following templates:

```jinja
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

### Components

Tera differs from Jinja2/Django template engines by offering first-class support for components.
This is fairly similar to macros with a JSX twist.

#### Defining a component

A component is defined using the `component`/`endcomponent` tags:

```jinja
{% component button(label, variant = "primary") %}
<button class="btn btn-{{variant}}">{{label}}</button>
{% endcomponent button %}
```

Components parameters can have default values (literals only, no expression) as well as an optional type that
can be inferred if there is a default value. The available types are:

- string
- bool
- integer
- float
- number (matches both integer and float)
- array
- map

The component above is closed: any templates using an argument not listed will error. You can make it open by adding a
spread operator:

```jinja
{% component button(label: string, variant = "primary", ...rest) %}
<button class="btn btn-{{variant}}">{{label}}</button>
{% endcomponent button %}
```

By doing that, any extra parameters other than `label` and `variant` will be collected into a map called `rest` that
can be used like any other maps.

Lastly, you can attach metadata to a component:

```jinja
{% component array(greeting: string) {"css": "./array.css"} %}
Hello
{% endcomponent %}
```

That metadata is available via the Rust API only, not in the templates.


#### Using a component

Components are registered globally: no need to import them. To avoid conflicts you can use
`.` in a component name to namespace them, eg `zola.button()`.

Let's say we defined the following components:

```jinja
{% component ui.button(label: string, variant: string = "primary", ...attrs) %}
    <button class="btn btn-{{variant}}">
      {{label}}{% if attrs.important %}!!{% endif %}
    </button>
{% endcomponent ui.button %}

{% component forms.input(name: string, label: string, required: bool = false) %}
    <label for="{{name}}">{{label}}{% if required %}*{% endif %}</label>
    <input type="text" name="{{name}}" {% if required %}required{% endif %}>
{% endcomponent forms.input %}

{% component ui.forms.widget(title: string) %}
    <div class="widget">
      <h3>{{title}}</h3>
      {{body}}
    </div>
{% endcomponent ui.forms.widget %}
```

We can use them like so:

```jinja
<div class="page">
  {{<ui.button label="Click me" variant="secondary" {...obj} />}}

  {{<forms.input name="email" label="Email Address" required={true} />}}

  {% <ui.forms.widget title> %}
    <p>This is a widget!</p>
    {{<ui.button label="Sign up" variant="primary"/>}}
  {% </ui.forms.widget> %}
</div>
```

Let's break this down.

`{{<forms.input name="email" label="Email Address" required={true} />}}` uses a self-closing tag with literals for kwargs.
For values other than strings, you need to use the `{..}` syntax like in JSX.

The cool part is:

```jinja
  {% <ui.forms.widget title> %}
    <p>This is a widget!</p>
    {{<ui.button label="Sign up" variant="primary" />}}
  {% </ui.forms.widget> %}
```
If you have a variable name that matches the argument (eg `title` in the example), you can use the shorthand approach to save some typing. If you look
at the definition above for `ui.forms.widget` you will see it's using `{{body}}` which is not defined anywhere: Tera will pass the
body of the component automatically as the `body` variable. You can of course nest it as much as you want.

If you are building with something like HTMX you can also re-render a single component from the `Tera` instance.

### Built-ins

#### Built-in filters
Tera has the following filters built-in:

##### safe
Marks a variable as safe: HTML will not be escaped anymore.
`safe` only works if it is the last filter of the expression:

- `{{ content | replace(from="Robert", to="Bob") | safe }}` will not be escaped
- `{{ content | safe | replace(from="Robert", to="Bob") }}` will be escaped

##### lower
Converts a string to lowercase.

##### upper
Converts a string to uppercase.

##### wordcount
Returns the number of words in a string.

##### capitalize
Returns the string with all its characters lowercased apart from the first char which is uppercased.

##### title
Capitalizes each word inside a sentence.

Example: `{{ value | title }}`

If value is "foo  bar", the output will be "Foo  Bar".

##### replace
Takes 2 mandatory string named arguments: `from` and `to`. It will return a string with all instances of
the `from` string replaced with the `to` string.

Example: `{{ name | replace(from="Robert", to="Bob")}}`

##### trim
Removes leading and trailing whitespace if the variable is a string.
Also takes an optional `pat` argument to trim by that pattern instead of whitespace:

Example: `{{ value | trim(pat="|") }}`

##### trim_start
Removes leading whitespace if the variable is a string.
Also takes an optional `pat` argument to trim by that pattern instead of whitespace:

Example: `{{ value | trim_start(pat="|") }}`

##### trim_end
Removes trailing whitespace if the variable is a string.
Also takes an optional `pat` argument to trim by that pattern instead of whitespace:

Example: `{{ value | trim_end(pat="|") }}`

##### truncate
Truncates a string to the indicated length. If the string has a smaller length than
the `length` argument, the string is returned as is.

Example: `{{ value | truncate(length=10) }}`

By default, the filter will add an ellipsis at the end if the text was truncated. You can
change the string appended by setting the `end` argument.
For example, `{{ value | truncate(length=10, end="") }}` will not append anything.

If you have the `unicode` feature enabled, the truncation will be done by graphemes rather than bytes.
Avoid using that filter with user strings if that feature is not enabled.

##### newlines_to_br
Replaces line breaks (`\n` or `\r\n`) with HTML line breaks (`<br>`).

Example: `{{ value | newlines_to_br }}`

If value is "Hello\r\nworld\n", the output will be "Hello<br>world<br>".

Note that if the template you are using it in is automatically escaped, you will
need to call the `safe` filter after `newlines_to_br`.

##### indent
Indents a string by injecting a prefix at the start of each line.
The `width` argument (default `4`) specifies how many spaces to insert per line.
If the `first` argument (default `false`) is set `true`, spaces are inserted for the first line.
If the `blank` argument (default `false`) is set `true`, spaces are inserted for blank/whitespace lines.

##### first
Returns the first element of an array.
If the array is empty, returns `None`.

##### last
Returns the last element of an array.
If the array is empty, returns `None`.

##### nth
Returns the nth element of an array.
If the array is empty, returns `None`.
It takes a required `n` argument, corresponding to the 0-based index you want to get.

Example: `{{ value | nth(n=2) }}`

##### join
Joins an array with a string.

Example: `{{ value | join(sep=" // ") }}`

If value is the array `['a', 'b', 'c']`, the output will be the string "a // b // c".

##### length
Returns the length of an array, an object, or a string.

##### reverse
Returns a reversed string or array.

##### escape_html
Escapes a string's HTML. Specifically, it makes these replacements:

- `&` is converted to `&amp;`
- `<` is converted to `&lt;`
- `>` is converted to `&gt;`
- `"` (double quote) is converted to `&quot;`
- `'` (single quote) is converted to `&#x27;`

##### escape_xml
Escapes XML special characters. Specifically, it makes these replacements:

- `&` is converted to `&amp;`
- `<` is converted to `&lt;`
- `>` is converted to `&gt;`
- `"` (double quote) is converted to `&quot;`
- `'` (single quote) is converted to `&apos;`

##### pluralize
Returns a plural suffix if the value is not equal to 1, or a singular suffix otherwise. The plural suffix defaults to `s` and the
singular suffix defaults to the empty string (i.e. nothing).

Example: `You have {{ num_messages }} message{{ num_messages | pluralize }}`

If num_messages is 1, the output will be You have 1 message. If num_messages is 2 the output will be You have 2 messages. You can
also customize the singular and plural suffixes with the `singular` and `plural` arguments to the filter:

Example: `{{ num_categories }} categor{{ num_categories | pluralize(singular="y", plural="ies") }}`

##### int
Converts a value into an integer.
The `base` argument can be used to specify how to interpret the number.
Bases of 2, 8, and 16 understand the prefix 0b, 0o, 0x, respectively.

##### float
Converts a value into a float.

##### str
Returns a string representation of the given value.

Example: `{{ value | str }}`

##### split
Splits a string into an array of strings, separated by a pattern given.
Example: `{{ path | split(pat="/") }}`

##### abs
Returns the absolute value

Example: `{{ negative_number | abs }}`

If negative_number is -1, the output will be 1. If num_messages is -2.0 the output will be 2.

##### round
Returns a number rounded following the method given. Default behaviour is to round to the nearest integer.
`ceil` and `floor` are available as alternative methods.
Another optional argument, `precision`, is available to select the precision of the rounding. It defaults to `0`, which will
round to the nearest integer for the given method.

Example: `{{ num | round }} {{ num | round(method="ceil", precision=2) }}`

##### sort
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

```jinja
{{ people | sort(attribute="name.1") }}
```

or by age:

```jinja
{{ people | sort(attribute="age") }}
```

##### unique
Removes duplicate items from an array.

##### filter

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

```jinja
{{ posts | filter(attribute="draft", value=true) }}
```

or by author name:

```jinja
{{ posts | filter(attribute="author.name", value="Vincent") }}
```

If `value` is not passed, it will drop any elements where the attribute is `null`.

##### group_by
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

```jinja
{{ posts | group_by(attribute="year") }}
```

or by author name:

```jinja
{% for name, author_posts in posts | group_by(attribute="author.name") %}
    {{ name }}
    {% for post in author_posts %}
        {{ post.year }}: {{ post.content }}
    {% endfor %}
{% endfor %}
```

Manipulating the hashmap produced by `group_by` in an arbitrary order requires additional steps to extract the keys into a separate array.

Example:

```jinja
{% set map = section.pages | group_by(attribute="year") %}
{% set_global years = [] %}
{% for year, ignored in map %}
    {% set_global years = [...years, year] %}
{% endfor %}
{% for year in years | reverse %}
    {% set posts = map[year] %}
{% endfor %}
```

##### map

Potentially retrieves an attribute from a list of objects and/or applies a filter to each element.

This has 3 arguments
 - `attribute`: specifies what attribute to retrieve from each element
 - `filter`: specifies a filter to apply to each element (or to the extracted attribute)
 - `args`: optional map of arguments to pass to the filter

At least one of `attribute` or `filter` must be provided.
If both are provided, the attribute is extracted first, then the filter is applied.

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

```jinja
{{ people | map(attribute="age") }}
{{ people | map(attribute="age", filter="str") }}
```
##### default
Returns the default value given only if the variable evaluated is not present in the context
and is therefore meant to be at the beginning of a filter chain if there are several filters.

Example: `{{ value | default(value=1) }}`

This is in most cases a shortcut for:

```jinja
{% if value %}{{ value }}{% else %}1{% endif %}
```

However, only the existence of the value in the context is checked. With a value that `if` would
evaluate to false (such as an empty string, or the number 0), the `default` filter will not attempt
replace it with the alternate value provided. For example, the following will produce
"I would like to read more !":

```jinja
I would like to read more {{ "" | default (value="Louise Michel") }}!
```

If you want `default` to check for truthiness instead, you can set the `boolean` parameter to `true`.

```jinja
I would like to read more {{ "" | default (value="Louise Michel", boolean=true) }}!
```
This will produce "I would like to read more Louise Michel".

##### keys
Returns an array of the keys of a map.

##### values
Returns an array of the values of a map.

##### pairs
Returns an array of `[key, value]` pairs from a map.

```jinja
{{ my_map | pairs }}
```

You can sort the pairs by key or by value using the `sort` filter after.

##### get
Accesses a value from an object when the key is not a Tera identifier.
Example: `{{ sections | get(key="posts/content") }}`

The `get` filter also has a `default` parameter which can be used to provide a return value when the `key` parameter is missing from the set being filtered.
Example: `{{ sections | get(key="posts/content", default="default") }}`

#### Built-in tests
Here are the currently built-in tests:

##### defined
Returns true if the given variable is defined.

##### undefined
Returns true if the given variable is undefined.

##### odd
Returns true if the given variable is an odd number.

##### even
Returns true if the given variable is an even number.

##### string
Returns true if the given variable is a string.

##### number
Returns true if the given variable is a number (integer or float).

##### integer
Returns true if the given variable is an integer.

##### float
Returns true if the given variable is a float.

##### map
Returns true if the given variable is a map.

##### array
Returns true if the given variable is an array.

##### bool
Returns true if the given variable is a bool.

##### none
Returns true if the given variable is `none`.

##### iterable
Returns true if the given variable can be iterated over in Tera (i.e. is an array, a map or a string).

##### starting\_with
Returns true if the given variable is a string and starts with the arg given.

Example:
```jinja
{% if path is starting_with(pat="x/") %}
    In section x
{% endif %}
```

##### ending\_with
Returns true if the given variable is a string and ends with the arg given.

##### containing
Returns true if the given variable contains the arg given.

The test works on:

- strings: is the arg a substring?
- arrays: is the arg given one of the members of the array?
- maps: is the arg given a key of the map?

Example:
```jinja
{% if username is containing(pat="xXx") %}
    Bad
{% endif %}
```

##### divisible_by
Returns true if the given expression is divisible by the arg given.

Example:
```jinja
{% if rating is divisible_by(divisor=2) %}
    Divisible
{% endif %}
```

#### Built-in functions

##### range

Returns an array of integers created using the arguments given.
There are 3 arguments, all integers:

- `end`: stop before `end`, mandatory
- `start`: where to start from, defaults to `0`
- `step_by`: the step between values, defaults to `1`, use a negative value to count down

##### throw
The template rendering will error with the given message when encountered.

There is only one string argument: `message` which is the message to display as the error



{% endraw %}
