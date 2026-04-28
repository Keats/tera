# v1 -> v2 migration guide

Tera v2 is a rewrite from scratch of Tera. 
A lot of things have changed for the better. 

## Template breaking changes

### Behaviour changes

- Changes to undefined variable access
  * `{{ hey }}` should error if hey is undefined
  * `{{ existing.hey }}` should error if hey is undefined but existing is
  * `{{ hey or 1 }}` should print 1
  * `{{ false and user.name }}` will not evaluate `user.name` and print `false`
  * `{% if hey or true %}` should be truthy
  * `{% if hey.other or true %}` should error if `hey` is not defined (currently truthy)
  * `{{ hey.other or 1 }}` should error if `hey` is not defined (currently prints "true")
- `{% if not_existing.field %}` errors if `not_existing` is undefined, we only allow one level of undefined-ness

### Filter/function/tests changes

Note that the built-in things requiring dependencies have been moved to a `tera-contrib` crate where 
they can be enabled one by one.

- tests now always take keyword arguments (kwargs)
- trim filters have been merged in trim/trim_start/trim_end with an optional `pat` argument for start/end rather than separate filters
- `int` and `float` filter do not have a default anymore
- `round` filter does not take a `common` method anymore, it's the default and should not be filled if needed
- `indent` filter now takes a `width` param rather than `prefix`
- `map`, `group_by` and `filter` filter will error if the attribute ends up being undefined on one of the value
- `as_str` has been renamed to `str`
- `divisibleby` has been renamed to `divisible_by`
- `escape` has been renamed to `escape_html`
- `linebreaksbr` has been renamed to `newlines_to_br`
- `object` test has been renamed to `map`
- `truncate` requires the `length` argument and does not default to 255 anymore
- ISO 8601 dates using format `1996-12-19T16:39:57-08:00` are not supported anymore for the input of date filter, you can use `1996-12-19T16:39:57[-08:00]` instead
- `addslashes`, `spaceless`, `get_env`, `concat` and `slice` filter have been removed (concat and slice are redundant with other features described later)
- `first`, `last` and `nth` now return None when the array is empty instead of an empty string.
- `unique` doesn't take arguments anymore

### Macros are gone

Yep completely gone. Nada.
They are replaced with components, described later in this document.

## Rust-side breaking changes

The way you define function/filters/tests in Rust has been greatly simplified and they can now access the context.
You can check the built-in ones to see how to define them and the crate documentation but here's an example:

```rust
// The first param is by default a `Value` but if you know what to expect, like a &str, in this case
// you can ask for it and the call will automatically error if the type doesn't match.
// You can also access the kwargs easily and cast their type, as well as accessing the context.
// No need to mention the name of the filter etc, it will automatically be added and the error will point
// to the right place.
// You can return any type that can be converted to a Value.
pub(crate) fn replace(val: &str, kwargs: Kwargs, _: &State) -> TeraResult<String> {
    let from = kwargs.must_get::<&str>("from")?;
    let to = kwargs.must_get::<&str>("to")?;

    Ok(val.replace(from, to))
}
```

Some functions will also only be available if the feature is enabled, such as `glob_fs` for globbing to load files.

Tera also now checks at compile-time that all functions/tests/filters/components are present and errors otherwise.
Make sure to register everything before adding the templates to the Tera instance.

## New things

### Map literals

You can now define maps in a template:

```j2
{% set m = {"a": 1, "b": 2} %}
```

and use inline maps anywhere you can use an expression.

### Spread

Now that we have maps, it's nice to be able to update them. If you've used JS, you will be familiar with that syntax:

```j2
{% set m = {...base, "d": 4} %}
```

This creates a new variable `m` with all the fields from `base` with the `d` value updated to `4`.

Spread also work for arrays:

```j2
{{ [0, ...numbers, 99] }}
```

### Slicing

You can now use slicing on your arrays without the need to use the filter, similar to Python slicing:

```j2
{{ numbers[0] }}
{{ numbers[-1] }}
{{ numbers[:-1] }}
{{ numbers[:2] }}
{{ numbers[1:2] }}
{{ numbers[0:2:2] }}
{{ numbers[::-1] }}
{{ product.name[-1] }}
{{ product.name[::-1] }}
{{ product.name[1:] }}
{{ product.name[:-1] }}
```

`-1` means the last item of the array and the syntax is `[start:stop:step]`, like Python.

### Optional chaining

Since we only allow one level of undefined-ness and we don't want to write a default filter for each access, we can use
optional chaining like in JS: `{{ a?.b?.c or "should print" }}`. This will try to load `a.b.c` but short-circuiting if any
value is null or undefined.

The syntax for optional arrays access is different from JS: `{{ a?['b']?.c or "should print" }}` is different from JS where
you would do `a?.['b']`.

### set blocks

You can use `set` with a body and apply filters to it:

```j2
{% set hero | upper | trans(lang="fr") %}
Hello {{ world }}
{% endset %}
```

### Ternary

You can now do `{{ "majeur" if age >= 18 else "mineur" }}`. Both if and else are required.

### Global context

You can now set a context on the Tera instance that will be passed automatically to all render calls.

### Components

Tera moves away from Jinja2 macros and adds first-class components. If you use macros heavily it's going to take some
work to change, but it should be nicer to use.

#### Defining a component

This is mostly the same as macros, except the block is called `component`/`endcomponent`:

```rust
{% component button(label: string, variant: string = "primary") %}
<button class="btn btn-{{variant}}">{{label}}</button>
{% endcomponent button %}
```

The other change is adding optional typing to component parameters (parameters with a default value can have their type inferred) and an optional component metadata that doesn't need to 
be explained here.

The component above is closed: any templates using an argument not listed will error. You can make it open by adding a 
spread operator:

```rust
{% component button(label: string, variant: string = "primary", ...rest) %}
<button class="btn btn-{{variant}}">{{label}}</button>
{% endcomponent button %}
```

By doing that, any extra parameters other than `label` and `variant` will be collected into a map called `rest` that
can be used like any other maps.


#### Using a component

That's where things change.

First, you don't need to import anything: components are registered globally. No more loading macros errors or using `self::` etc.
Second, how you call the components has completely changed, inspired by https://jinjax.scaletti.dev/. Now I didn't go fully
JSX but it's kind of a mix between Jinja2 and JSX.

First some definition:

```j2
{% component ui.button(label: string, variant: string = "primary", ...attrs) %}
    <button class="btn btn-{{variant}}">{{label}}{% if attrs.important %}!!{% endif %}</button>
{% endcomponent ui.button %}

{% component forms.input(name: string, label: string, required: bool = false) %}
    <label for="{{name}}">{{label}}{% if required %}*{% endif %}</label>
    <input type="text" name="{{name}}" {% if required %}required{% endif %}>
{% endcomponent forms.input %}

{% component ui.forms.widget(title: string) %}
    <div class="widget">
      <h3>{{title}}</h3>
      {{body}}
    </div>
{% endcomponent ui.forms.widget %}
```

And then actually calling it:

```j2
<div class="page">
  {{<ui.button label="Click me" variant="secondary" {...obj} />}}

  {{<forms.input name="email" label="Email Address" required={true}/>}}

  {% <ui.forms.widget title> %}
    <p>This is a widget!</p>
    {{<ui.button label="Sign up" variant="primary"/>}}
  {% </ui.forms.widget> %}
</div>
```

Let's break this down.

`{{<forms.input name="email" label="Email Address" required={true}/>}}` uses a self-closing tag with literals for kwargs.
For values other than strings, you need to use the `{..}` syntax like in JSX.

The cool part is:

```j2
  {% <ui.forms.widget title> %}
    <p>This is a widget!</p>
    {{<ui.button label="Sign up" variant="primary"/>}}
  {% </ui.forms.widget> %}
```
If you have a variable name that matches the argument (eg `title` in the example), you can use the shorthand approach to save some typing. If you look
at the definition above for `ui.forms.widget` you will see it's using `{{body}}` which is not defined anywhere: Tera will pass the
body of a component automatically as the `body` variable. You can of course nest it as much as you want.

If you are building with something like HTMX you can also re-render a single component from the `Tera` instance.

## Custom delimiters

The default delimiters `{{ }}`, `{% %}` and `{# #}` are the same as in Tera v1 but you can now customise them via `Tera::set_delimiters`

## Performance

It will depend on what you are doing inside the template and the size of your context but it is about 2x faster than Tera v1.

## Better error messages

"Borrowing" the error message structure from Rust:

```
error: Field `undefined_var` is not defined
 --> included:1:3
  |
1 | {{ undefined_var }}
  |    ^^^^^^^^^^^^^

note: called from tpl:1:11
  |
1 | {% include "included" %}
  |            ^^^^^^^^^^
```
