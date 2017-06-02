+++
template = "docs.html"
sort_by = "order"
insert_anchor = "right"
+++

# Welcome to Tera

Tera is an open-source template engine written in Rust based on Jinja2 and Django templates. It will feel familiar if you have
used Twig or Liquid as well.


```jinja2
<title>{% block title %}{% endblock %}</title>
<ul>
{% for user in users %}
  <li><a href="{{ user.url }}">{{ user.username }}</a></li>
{% endfor %}
</ul>
```
