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
</ul>
```

## Documentation
API documentation is available on [docs.rs](https://docs.rs/crate/tera/).

Tera documentation is available on its [site](https://tera.netlify.com/docs/installation/).

## SemVer
This project follows SemVer only for the public API, public API here meaning functions appearing in the docs.
Some features, like accessing the AST, are also available but breaking changes in them can happen in minor versions.
