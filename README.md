# Tera

[![Build Status](https://travis-ci.org/Keats/tera.svg)](https://travis-ci.org/Keats/tera)


## Introduction
Tera is a template engine based on [Jinja2](http://jinja.pocoo.org/) and the [Django template language](https://docs.djangoproject.com/en/1.9/topics/templates/).

It is subject to lots of API changes as users provide feedback.

While Tera is inspired by the engines above, it doesn't have the backward compatibility to maintain and we can improve on those if possible. One of the goal is to avoid putting too much logic in the templates so it's closer to the Django ones in that respect, except it has math operations built-in.

Example of a template file:

```jinja
<html>
  <head>
    <title>{{ product.name }}</title>
  </head>
  <body>
    <h1>{{ product.name }} - {{ product.manufacturer }}</h1>
    <p>{{ product.summary }}</p>
    <p>£{{ product.price * 1.20 }} (VAT inc.)</p>
    {% if friend_reviewed %}
      <p>Look at reviews from your friends {{ username }}</p>
      {% if number_reviews > 10 || show_more %}
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
Tera will load and parse all the templates in the given directory.

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
let tera = Tera::new("templates/**/*");
```

Tera will panic on invalid templates which means you should add template compilation as a build step when compiling. Have a look at that [page to learn more about build script](http://doc.crates.io/build-script.html).

This step is also meant to only be ran once, so you can use something like [lazy_static](https://crates.io/crates/lazy_static) to have the `tera` variable as a global static in your app.

If no errors happened while parsing any of the files, you can now render a template like so:

```rust
use tera::Context;

let mut context = Context::new();
context.add("product", &product);
context.add("vat_rate", &0.20);

tera.render("products/product.html", context);
```
Notice that the name of the template is based on the root of the template directory given to the Tera instance.
`Context` takes any primitive value or a struct that implements the `Serialize` trait from `serde_json`. 

If the data you want to render implements the `Serialize` trait, you can bypass the context and render the value directly:

```rust
// product here is a struct with a `name` field
tera.value_render("products/product.html", &product);

// in product.html
{{ name }}
```
Note that this method only works for objects that would be converted to JSON objects, like structs and maps.
 

## Template writer documentation
### Variables
You can access variables of the context by using the `{{ my_variable_name }}` construct. 
You can access attributes by using the dot (`.`) like `{{ product.name }}`.
You can access specific members of an array or tuple by using the `.i` notation where `i` is a zero-based index.

You can also do some maths: `{{ product.price + 10 }}`. If `product.price` is not a number type, the `render` method will return an error.

### If
Similar to the if in Rust, you can have several conditions and also use `elif`
and `else`:

```jinja
{% if price < 10 || always_show %}
   Price is {{ price }}.
{% elif price > 1000 %}
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

If `my_var` is defined, the `if` branch will be rendered. Otherwise, the `else`
branch will be rendered.

Every `if` statement has to end with an `endif` tag.

### For
Loop over items in a array:
```jinja
{% for product in products %}
  {{loop.index}}. {{product.name}}
{% endfor %}
```
A few special variables are available inside for loops like in jinja2:

- `loop.index`: current iteration 1-indexed
- `loop.index0`: current iteration 0-indexed
- `loop.first`: whether this is the first iteration
- `loop.last`: whether this is the last iteration

The `for` statement has to end with a `endfor` tag.

### Raw
Allow you to ignore texts that Tera would try to render otherwise.
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
Tera uses the same kind of inheritance as Jinja2 and django templates: you define a base template and extends it in child templates.

#### Base template
A base template typically contains the basic html structure as well as several `blocks` that can contain placeholders.
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
The difference with Jinja being that `endblock` tags must be named.
This defines 4 `block` tag that child templates can override. The `head` and `footer` block contains some html already which will be rendered if they are not overriden.

#### Child template
Again, straight from jinja2 docs:

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

When trying to render that template, Tera will see that it depends on a parent template and will render it first, filling the blocks as it encounters them in the base template.

#### Include
You can include a template to be rendered using the current context with the `include` tag.

```jinja
{% include "included.html" %}
```

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

Here are the currently implemented testers:

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

### Filters
Variables can be modified by filters. 
Filters are separated from the variable by a pipe symbol (`|`) and may have named arguments in parentheses. Multiple filters can be chained: the output of one filter is applied to the next.

For example, `{{ name | lower | replace(from="doctor", to="Dr.") }}` will take a variable called `name` and make it lowercase and then replace instances of `doctor` by `Dr.`. It's equivalent to `replace(lower(name), from="doctor", to="Dr.")` as a function.

Note that calling filters on a incorrect type like trying to capitalize an array will result in a error.

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

If value is "I'm using Tera", the output will be "I\'m using Tera"

#### slugify
Transform a string into ASCII, lowercase it, trim it, converts spaces to hyphens and remove all characters that are not numbers, lowercase letters or hyphens.

Example: `{{ value | slugify}}`
If value is "-Hello world! ", the output will be "hello-world".

#### title
Capitalizes each word inside a sentence.

Example: `{{ value | title}}`
If value is "foo  bar", the output will be "Foo  Bar".

#### striptags
Tries to remove HTML tags from input. Does not guarantee well formed output if input is not valid HTML.

Example: `{{ value | striptags}}`
If value is "<b>Joel</b>", the output will be "Joel" 

#### first
Returns the first element of an array.
If the array is empty, returns empty string;

#### last
Returns the last element of an array.
If the array is empty, returns empty string;

#### join
Joins an array with a string.

Example: `{{ value|join:" // " }}`

If value is the array ['a', 'b', 'c'], the output will be the string "a // b // c".

#### length
Returns the length of an array or a string, 0 if the value is not an array.
// TODO: return an error instead to be consistent?

#### reverse
Returns a reversed string or array

#### urlencode
Percent-encodes a string.

Example: `{{ value | urlencode }}`
If value is "/foo?a=b&c=d", the output will be "/foo%3Fa%3Db%26c%3Dd".

Takes an optional argument of characters that shouldn't be percent-encoded (`/` by default). So, to encode slashes as well, you can do `{{ value | urlencode(safe: "") }}`. 

#### pluralize
Returns a suffix if the value is greater or equal than 2. Suffix defaults to `s`

Example: `You have {{ num_messages }} message{{ num_messages|pluralize }}`
If num_messages is 1, the output will be You have 1 message. If num_messages is 2 the output will be You have 2 messages.
You can specify the suffix as an argument that way: `{{ num_messages|pluralize(suffix="es") }}`


#### round
Returns a number rounded following the method given. Default method is `common` which will round to the nearest integer.
`ceil` and `floor` are available as alternative methods.

Example: `{{ num | round }} {{ num | round(method="ceil") }}`


#### escape
Escapes a string's HTML. Specifically, it makes these replacements:

- < is converted to &lt;
- > is converted to &gt;
- ' (single quote) is converted to &#39;
- " (double quote) is converted to &quot;
- & is converted to &amp;

### Macros
Macros are a simple way to reuse template bits. Think of them as functions that you can call that return some text.

Macros are defined as follows:

```jinja2
{% macro input(label, type) %}
    <label>
        {{ label }}
        <input type="{{type}}" />
    </label>
{% endmacro hello_world %}
```

You need to import the file containing the macros in order to use them:

```jinja2
{% import "macros.html" as macros %}
```
You can name that file namespace (`macros` in the example) anything you want.
You can call a macro the following way:

```jinja2
{{ macros::input(label="Name", type="text") }}
```
Do note that macros, like other functions in Tera, require keyword arguments.
If you are trying to call a macro defined in the same file, you will need to use the `{{ self::my_macro() }}`` syntax to call it the file its defined in.

Macros can be called recursively but there is no limit to recursion so make sure you macro ends.

There are a few restrictions to the content inside macros: no macros definitions or blocks are allowed.
