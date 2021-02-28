+++
+++

```jinja2
<title>{% useblock title %}</title>
<ul>
{% for user in users -%}
  <li><a href="{{ user.url }}">{{ user.username }}</a></li>
{%- endfor %}
</ul>
```
