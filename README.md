# Tera

[![Build Status](https://travis-ci.org/Keats/tera.svg)](https://travis-ci.org/Keats/tera)

Current release API docs are available on [docs.rs](https://docs.rs/tera).
This project follows semver only for the public API, public API here meaning functions appearing in the docs.
Some features, like accessing the AST, are also available but breaking changes in them can happen in minor versions.

## Introduction
Tera is a template engine based on [Jinja2](http://jinja.pocoo.org/) and the [Django template language](https://docs.djangoproject.com/en/1.9/topics/templates/).

While Tera is inspired by the engines above, it doesn't aim to be a complete port of one of the other.

Example of a simple template file:

```jinja
<html>
  <head>
    <title>{{ product.name }}</title>
  </head>
  <body>
    <h1>{{ product.name | upper }} - {{ product.manufacturer }}</h1>
    <p>{{ product.summary }}</p>
    <p>£{{ product.price * 1.20 }} (VAT inc.)</p>
    {% if friend_reviewed %}
      <p>Look at reviews from your friends {{ username }}</p>
      {% if number_reviews > 10 or show_more %}
        <p>All reviews</p>
        {% for review in reviews %}
          <h3>{{review.title}}</h3>
          {% for paragraph in review.paragraphs %}
            <p>{{ paragraph }}</p>
          {% endfor %}
        {% endfor %}
      {% elif number_reviews == 1 %}
        <p>Only one review</p>
      {% endif %}
    {% else %}
      <p>None of your friend reviewed this product</p>
    {% endif %}
    <button>Buy!</button>
  </body>
</html>
```

## Usage
The primary method of using Tera will load and parse all the templates in the given glob.

Let's take the following directory as example.
```bash
templates/
  hello.html
  index.html
  products/
    product.html
    price.html
```

Assuming the rust file is at the same level as the `templates` folder, we would parse the templates that way:

```rust
use tera::Tera;

// Use globbing
let tera = compile_templates!("templates/**/*");
```

The `compile_templates!` macro will try to parse all files found in the glob. If errors are encountered, it will print them and exit the process.

If you don't want to exit the process on errors, you can call the `Tera::new` method and handle errors directly.
Compiling templates is a step is also meant to only be ran once: use something like [lazy_static](https://crates.io/crates/lazy_static) 
to have the `tera` variable as a global static in your app. See `examples/basic.rs` for an example.

If no errors happened while parsing any of the files, you can now render a template like so:

```rust
use tera::Context;

let mut context = Context::new();
context.add("product", &product);
context.add("vat_rate", &0.20);

tera.render("products/product.html", &context);
```
Notice that the name of the template is based on the root of the template directory given to the Tera instance.
`Context` takes any primitive value or a struct that implements the `Serialize` trait from `serde_json`. You can also merge 2 
`Context` by using the `Context::extend` method.

If the data you want to render implements the `Serialize` trait, you can bypass the context and render the value directly:

```rust
// product here is a struct with a `name` field
tera.render("products/product.html", &product);

// in product.html
{{ name }}
```
Note that this method only works for objects that would be converted to JSON objects, like structs and maps.
 

Want to render a single template? For example a user given one? Tera provides the `one_off` function for that.

```rust
// The last parameter is whether we want to autoescape the template or not.
// Should be true in 99% of the cases for HTML
let context = Context::new()
// add stuff to context
let result = Tera::one_off(user_tpl, &context, true);
// Or use a struct
let result = Tera::one_off(user_tpl, &user, true);
```


### Autoescaping
By default, autoescaping is turned on for files ending in `.html`, `.htm` and `.xml`.
You can change that by calling `Tera::autoescape_on` with a Vec of suffixes. Suffixes don't have to be extensions.

```rust
let mut tera = compile_templates!("templates/**/*");
tera.autoescape_on(vec!["email.j2", ".sql"]);
```
Note that calling `autoescape_on` will remove the defaults. If you want to completely disable autoescaping, simply
call `tera.autoescape_on(vec![]);`.


## Template writer documentation
### Variables
You can access variables of the context by using the `{{ my_variable_name }}` construct. 
You can access attributes by using the dot (`.`) like `{{ product.name }}`.
You can access specific members of an array or tuple by using the `.i` notation where `i` is a zero-based index.

You can also do some maths: `{{ product.price + 10 }}`. If `product.price` is not a number type, the `render` method will return an error.

### If
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

### For
Loop over items in a array:
```jinja
{% for product in products %}
  {{loop.index}}. {{product.name}}
{% endfor %}
```
A few special variables are available inside for loops:

- `loop.index`: current iteration 1-indexed
- `loop.index0`: current iteration 0-indexed
- `loop.first`: whether this is the first iteration
- `loop.last`: whether this is the last iteration

Every `for` statement has to end with an `endfor` tag.

### Raw
Tera will consider all text inside the `raw` block as a string and won't try to
render what's inside. Useful if you have text that contains Tera delimiters.
```jinja
{% raw %}
  Hello {{ name }}
{% endraw %}
```
would be rendered:
```jinja
Hello {{ name }}
```

### Inheritance
Tera uses the same kind of inheritance as Jinja2 and Django templates: 
you define a base template and extends it in child templates through blocks.
There can be multiple levels of inheritance (i.e. A extends B that extends C).

#### Base template
A base template typically contains the basic document structure as well as 
several `blocks` that can have content.

For example, here's a `base.html` almost copied from the jinja documentation:

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
The only difference with Jinja2 is that the `endblock` tags have to be named.
This `base.html` template defines 4 `block` tag that child templates can override. 
The `head` and `footer` block have some content already which will be rendered if they are not overridden.

#### Child template
Again, straight from Jinja2 docs:

```jinja
{% extends "base.html" %}
{% block title %}Index{% endblock title %}
{% block head %}
    {{ super() }}
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

To indicate inheritance, you have use the `extends` tag as the first thing in the file followed by the name of the template you want
to extend.
The `{{ super() }}` variable call tells Tera to render the parent block there.

Nested blocks are valid in Tera, consider the following templates:

```jinja2
// grandparent
{% block hey %}hello{% endblock hey %}

// parent
{% extends "grandparent" %}
{% block hey %}hi and grandma says {{ super() }} {% block ending %}sincerely{% endblock ending %}{% endblock hey %}

// child
{% extends "parent" %}
{% block hey %}dad says {{ super() }}{% endblock hey %}
{% block ending %}{{ super() }} with love{% endblock ending %}
```
The block `ending` is nested in the `hey` block. Rendering the `child` template will do the following:

- Find the first base template: `grandparent`
- See `hey` block in it and checks if it is in `child` and `parent` template
- It is in `child` so we render it, it contains a `super()` call so we render the `hey` block from `parent`, 
which also contains a `super()` so we render the `hey` block of the `grandparent` template as well
- See `ending` block in `child`, render it and also renders the `ending` block of `parent` as there is a `super()`

The end result of that rendering (not counting whitespace) will be: "dad says hi and grandma says hello sincerely with love".

#### Include
You can include a template to be rendered using the current context with the `include` tag.

```jinja
{% include "included.html" %}
```

Tera doesn't offer passing a custom context to the `include` tag. If you want to do that, use macros.

### Macros
Macros are a simple way to reuse template bits. Think of them as functions that you can call and return some text.

They are defined as follows:

```jinja2
{% macro input(label, type) %}
    <label>
        {{ label }}
        <input type="{{type}}" />
    </label>
{% endmacro hello_world %}
```

In order to use them, you need to import the file containing the macros:

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
If you are trying to call a macro defined in the same file or itself, you will need to use the `self` namespace.
Macros can be called recursively but there is no limit to recursion so make sure you macro ends.

Here's an example of a recursive macro:

```jinja2
{% macro factorial(n) %}
  {% if n > 1 %}{{ n }} - {{ self::factorial(n=n-1) }}{% else %}1{% endif %}
{% endmacro factorial %}
```

Macros body can contain all normal Tera syntax with the exception of macros definition, `block` and `extends`.

### Tests

Tests can be used against a variable to check some condition on the variable.
Perhaps the most common use of variable tests is to check if a variable is
defined before its use to prevent run-time errors. Tests are made against
variables in `if` blocks using the `is` keyword. For example, to test if `user`
is defined, you would write:

```
{% if user is defined %}
... do something with user ...
{% else %}
... don't use user here ...
{% end %}
```
Note that testers allow expressions, so the following is a valid test as well:

```
{% if my_number + 1 is odd %}
 blabla
{% endif %}
```

Tests are functions with the `fn(Option<Value>, Vec<Value>) -> Result<bool>` type and custom ones can be
registered like so:

```rust
tera.register_tester("odd", testers::odd);
```

Here are the currently built-in testers:

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
{% if rating is divisibleby 2 %}
    Divisible
{% endif %}
```

#### iterable
Returns true if the given variable can be iterated over in Tera (ie is an array/tuple).

### Filters
Variables can be modified by filters before being rendered. 
Filters are separated from the variable by a pipe symbol (`|`) and may have named arguments in parentheses. 
Multiple filters can be chained: the output of one filter is applied to the next.

For example, `{{ name | lower | replace(from="doctor", to="Dr.") }}` will take a variable called `name`, 
make it lowercase and then replace instances of `doctor` by `Dr.`. 
It is equivalent to `replace(lower(name), from="doctor", to="Dr.")` if we were to look at it as functions.

Note that calling filters on a incorrect type like trying to capitalize an array will result in a error.

Filters are functions with the `fn(Value, HashMap<String, Value>) -> Result<Value>` type and custom ones can be added
like so:

```rust
tera.register_filter("upper", string::upper);
```

Tera has currently the following filters built-in:

#### lower
Lowercase a string

#### wordcount
Returns number of words in a string

#### capitalize
Returns the string with all its character lowercased apart from the first char which is uppercased.

#### replace
Takes 2 mandatory string named arguments: `from` and `to`. It will return a string with all instances of 
the `from` string with the `to` string.

Example: `{{ name | replace(from="Robert", to="Bob")}}`

#### addslashes
Adds slashes before quotes.

Example: `{{ value | addslashes }}` 

If value is "I'm using Tera", the output will be "I\'m using Tera".

#### slugify
Transform a string into ASCII, lowercase it, trim it, converts spaces to hyphens and 
remove all characters that are not numbers, lowercase letters or hyphens.

Example: `{{ value | slugify}}`

If value is "-Hello world! ", the output will be "hello-world".

#### title
Capitalizes each word inside a sentence.

Example: `{{ value | title}}`

If value is "foo  bar", the output will be "Foo  Bar".

#### striptags
Tries to remove HTML tags from input. Does not guarantee well formed output if input is not valid HTML.

Example: `{{ value | striptags}}`

If value is "<b>Joel</b>", the output will be "Joel".

#### first
Returns the first element of an array.
If the array is empty, returns empty string.

#### last
Returns the last element of an array.
If the array is empty, returns empty string.

#### join
Joins an array with a string.

Example: `{{ value|join:" // " }}`

If value is the array `['a', 'b', 'c']`, the output will be the string "a // b // c".

#### length
Returns the length of an array or a string, 0 if the value is not an array.
// TODO: return an error instead to be consistent?

#### reverse
Returns a reversed string or array.

#### urlencode
Percent-encodes a string.

Example: `{{ value | urlencode }}`

If value is "/foo?a=b&c=d", the output will be "/foo%3Fa%3Db%26c%3Dd".

Takes an optional argument of characters that shouldn't be percent-encoded (`/` by default). 
So, to encode slashes as well, you can do `{{ value | urlencode(safe="") }}`. 

#### pluralize
Returns a suffix if the value is greater or equal than 2. Suffix defaults to `s`

Example: `You have {{ num_messages }} message{{ num_messages|pluralize }}`

If num_messages is 1, the output will be You have 1 message. If num_messages is 2 the output will be You have 2 messages.
You can specify the suffix as an argument that way: `{{ num_messages|pluralize(suffix="es") }}`

#### round
Returns a number rounded following the method given. Default method is `common` which will round to the nearest integer.
`ceil` and `floor` are available as alternative methods.
Another optional argument, `precision`, is available to select the precision of the rounding. It defaults to `0`, which will
round to the nearest integer for the given method.

Example: `{{ num | round }} {{ num | round(method="ceil", precision=2) }}`

#### filesizeformat
Returns a human-readable file size (i.e. '110 MB') from an integer.

Example: `{{ num | filesizeformat }}`

#### date
Parse a timestamp into a date(time) string. Defaults to `YYYY-MM-DD` format.
Time formatting syntax is inspired from strftime and a full reference is available 
on [chrono docs](https://lifthrasiir.github.io/rust-chrono/chrono/format/strftime/index.html).

Example: `{{ ts | date }} {{ ts | date(format="%Y-%m-%d %H:%M")`

#### escape
Escapes a string's HTML. Specifically, it makes these replacements:

- & is converted to `&amp;`
- < is converted to `&lt;`
- > is converted to `&gt;`
- " (double quote) is converted to `&quot;`
- ' (single quote) is converted to `&#x27;`
- / is converted to `&#x27;`
- `` ` `` is converted to `&#96;`


### Filter sections
Whole sections can also be processed by filters if they are encapsulated in `{% filter name %}` and `{% endfilter %}`
tags where `name` is the name of the filter.

Example:
```jinja2
{% filter upper %}
    Hello
{% endfilter %}
```

This example transforms the text `Hello` in all upper-case (`HELLO`).

Note that calling filters on an incorrect type like trying to capitalize an array will result in a error.

If the return type of the filter is not a string it will be converted to a string using the JSON format.

Filters are functions with the `fn(Value, HashMap<String, Value>) -> Result<Value>` type and custom ones can be added
like so:

```rust
tera.register_filter("upper", string::upper);
```
Filter functions for regular filters can also be used for filter sections.


## Accessing the AST
Tera gives access to the AST of each template but the functions required is hidden
from the docs at the current time.
See `examples/ast.rs` for an example on how to get the AST for a given template.

The AST is not considered public and breaking changes could happen in minor versions.
