# Tera

[![Build Status](https://travis-ci.org/Keats/tera.svg)](https://travis-ci.org/Keats/tera)

## TODOs:
- filters

Other:
- move to gitlab once CI for stable/beta/nightly is figured out


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

## Template writer documentation
### Variables
You can access variables of the context by using the `{{ my_variable_name }}` construct. You can access attributes by using the dot (`.`) like `{{ product.name }}`.
You can also do some maths: `{{ product.price + 10 }}`. If `product.price` is not a number type, the `render` method will return an error.

### If
Similar to the if in Rust, you can have several conditions and also use `elif` and `else`:

```jinja`
{% if price < 10 || always_show %}
   Price is {{ price }}.
{% elif price > 1000 %}
   That's expensive!
{% else %}
    N/A
{% endif %}
```
The `if` statement has to end with a `endif` tag.

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
This defines 4 `block` tag that child templates can override. The `head` and `footer` block contains some html already which will be rendered if they are not overrident.

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
