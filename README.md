# Tera

[![Build Status](https://travis-ci.org/Keats/tera.svg)](https://travis-ci.org/Keats/tera)
[![Build status](https://ci.appveyor.com/api/projects/status/omd2auu2e9qc8ukd?svg=true)](https://ci.appveyor.com/project/Keats/tera)
[![Crates.io](https://img.shields.io/crates/v/tera.svg)](https://crates.io/crates/tera)
[![Docs](https://docs.rs/tera/badge.svg)](https://docs.rs/crate/tera/)

Tera is a template engine inspired by [Jinja2](http://jinja.pocoo.org/) and the [Django template language](https://docs.djangoproject.com/en/1.9/topics/templates/).

```jinja2
<title>{% block title %}{% endblock title %}</title>
<ul>
{% for user in users %}
  <li><a href="{{ user.url }}">{{ user.username }}</a></li>
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
  {{loop.index}}. {{product.name}}
{% endfor %}
```
`key` and `value` can be named however you want, they just need to be separated with a comma.

If you are iterating on an array, you can also apply filters to the container:

```jinja2
{% for product in products | reverse %}
  {{loop.index}}. {{product.name}}
{% endfor %}
```

### Raw
Tera will consider all text inside the `raw` block as a string and won't try to
render what's inside. Useful if you have text that contains Tera delimiters.
```jinja2
{% raw %}
  Hello {{ name }}
{% endraw %}
```
would be rendered:
```jinja2
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
The only difference with Jinja2 is that the `endblock` tags have to be named.
This `base.html` template defines 4 `block` tag that child templates can override. 
The `head` and `footer` block have some content already which will be rendered if they are not overridden.

#### Child template
Again, straight from Jinja2 docs:

```jinja2
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
Macros need to be defined in a separate file and imported to be useable.

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
The `self` namespace can only be used in macros.
Macros can be called recursively but there is no limit to recursion so make sure your macro ends.

Here's an example of a recursive macro:

```jinja2
{% macro factorial(n) %}
  {% if n > 1 %}{{ n }} - {{ self::factorial(n=n-1) }}{% else %}1{% endif %}
{% endmacro factorial %}
```

## Documentation
API documentation is available on [docs.rs](https://docs.rs/crate/tera/).

Tera documentation is available on its [site](https://tera.netlify.com/docs/installation/).

## SemVer
This project follows SemVer only for the public API, public API here meaning functions appearing in the docs.
Some features, like accessing the AST, are also available but breaking changes in them can happen in minor versions.
