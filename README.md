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

### For

### Inheritance
