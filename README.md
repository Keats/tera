# Tera

[![Actions Status](https://github.com/Keats/tera/workflows/ci/badge.svg)](https://github.com/Keats/tera/actions)
[![Crates.io](https://img.shields.io/crates/v/tera.svg)](https://crates.io/crates/tera)
[![Docs](https://docs.rs/tera/badge.svg)](https://docs.rs/crate/tera/)
[![Gitter](https://badges.gitter.im/Tera-templates/community.svg)](https://gitter.im/Tera-templates/community?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)

Tera is a template engine inspired by [Jinja2](http://jinja.pocoo.org/) and the [Django template language](https://docs.djangoproject.com/en/3.1/topics/templates/).

```jinja2
<title>{% block title %}{% endblock title %}</title>
<ul>
{% for user in users %}
  <li><a href="{{ user.url }}">{{ user.username }}</a></li>
{% endfor %}
</ul>
```

It does not aim to be 100% compatible with them but has many of the Jinja2/Django filters and testers.

## Documentation
API documentation is available on [docs.rs](https://docs.rs/crate/tera/).

Tera documentation is available on its [site](http://keats.github.io/tera/docs).

## SemVer
This project follows SemVer only for the public API, public API here meaning functions appearing in the docs.
Some features, like accessing the AST, are also available but breaking changes in them can happen in any versions.
