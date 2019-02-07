+++
+++

```jinja2
<title>{% block title %}{% endblock title %}</title>
<ul>
{% for user in users -%}
  <li><a href="{{ user.url }}">{{ user.username }}</a></li>
{%- endfor %}
</ul>
```
