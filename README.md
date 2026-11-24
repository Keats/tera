# Tera

For v1 users: see [migration guide](./MIGRATION.md).

Tera is a template engine inspired by [Jinja2](http://jinja.pocoo.org/) and the [Django template language](https://docs.djangoproject.com/en/6.0/topics/templates/).

```jinja2
<title>{% block title %}{% endblock title %}</title>
<ul>
{% for user in users %}
  <li><a href="{{ user.url }}">{{ user.username }}</a></li>
{% endfor %}
</ul>
```

It intentionally deviates from Jinja2/Django in many ways, only the overall look and feel is similar.

## Documentation
API documentation is available on [docs.rs](https://docs.rs/crate/tera/).

Tera documentation is available on its [site](http://keats.github.io/tera/).

See the [here](CHANGELOG.md) for changes to tera and [there for tera-contrib](tera-contrib/CHANGELOG.md).

## AI

AI contributions are allowed but a human is still responsible for it: you should understand the changes
in a PR and be able to explain them without pasting questions in your LLM.
Do not open an issue/PR without verifying it yourself.
Do not create a PR to improve performance of something that will never be a realistic use case.