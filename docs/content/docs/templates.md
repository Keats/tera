+++
order = 10
+++

# Templates

A Tera template is just a text file where variables and expressions get replaced with values
when it is rendered. The syntax is based on Jinja2 and Django templates.

There are 3 kinds of delimiter and those cannot be changed:

- `{{` and `}}` for expressions
- `{%` or `{%-` and `%}` or `-%}` for statements
- `{#` and `#}` for comments

## Literals

Tera has a few literals that can be used:

- booleans: `true` and `false`
- integers
- floats
- strings: text delimited by `""`, `''` or backticks

## Variables

Variables are defined by the context given when rendering a template.

You can render a variable by using the `{{ name }}` construct and attributes
can be accessed by using the dot (`.`) like `{{ product.name }}`. 
Specific members of an array or tuple are accessed by using the `.i` notation, where i is a zero-based index.

Trying to access or render a variable that doesn't exist will result in an error.

A magical variable is available in every template if you want to print the current context: `__tera_context`.

## Expressions

Tera allows expressions almost everywhere.

### Math
You can do some basic math in Tera but it shouldn't be abused other than the occasional `+ 1` or similar.
Math operations are only allowed with numbers, using them on any other kind of values will result in an error.
You can use the following operators:

- `+`: adds 2 values together, `{{ 1 + 1 }}` will print `2`
- `-`: performs a substraction, `{{ 2 - 1 }}` will print `1`
- `/`: performs a division, `{{ 10 / 2 }}` will print `5`
- `*`: performs a multiplication, `{{ 5 * 2 }}` will print `10`
- `%`: performs a modulo, `{{ 2 % 2 }}` will print `0`

The priority of operations is the following, from lowest to highest:

- `+` and `-`
- `*` and `/` and `%`

### Comparisons

- `==`: checks whether the values are equal
- `!=`: checks whether the values are different
- `>=`: true if the left value is equal or greater to the right one
- `<=`: true if the right value is equal or greater to the left one
- `>`: true if the left value is greater than the right one
- `<`: true if the right value is greater than the left one

### Logic

- `and`: true if the left and right operands are true
- `or`: true if the left or right operands are true
- `not`: negate a statement

## Filters

You can modify variables using **filters**. 
Filters are separated from the variable by a pipe symbol (|) and may have named arguments in parentheses. 
Multiple filters can be chained: the output of one filter is applied to the next.

For example, `{{ name | lower | replace(from="doctor", to="Dr.") }}` will take a variable called name, make it lowercase and then replace instances of `doctor` by `Dr.`. 
It is equivalent to `replace(lower(name), from="doctor", to="Dr.")` if we were to look at it as functions.

Calling filters on a incorrect type like trying to capitalize an array or using invalid types for arguments will result in a error.

Filters are functions with the `fn(Value, HashMap<String, Value>) -> Result<Value>` definition and custom ones can be added like so:

```rust
tera.register_filter("upper", string::upper);
```

While filters can be used in math operations, they will have the lowest priority and therefore might not do what you expect:


```css
{{ 1 + a | length }}
// is equal to
{{ (1 + a) | length } // this will probably error

// This will do what you wanted initially
{{ a | length + 1 }}
```
Tera has many [built-in filters](./docs/templates.md#built-in-filters) that you can use.

### Filter sections

Whole sections can also be processed by filters if they are encapsulated in `{% filter name %}` and `{% endfilter %}`
tags where `name` is the name of the filter:

```jinja2
{% filter upper %}
    Hello
{% endfilter %}
```

This example transforms the text `Hello` in all upper-case (`HELLO`).

## Tests

Tests can be used against an expression to check some condition on it and 
are made in `if` blocks using the `is` keyword. 
For example, you would write the following to test if an expression is odd:

```jinja2
{% if my_number is odd %}
 Odd
{% endif %}
```

Tests are functions with the `fn(Option<Value>, Vec<Value>) -> Result<bool>` definition and custom ones can be added like so:

```rust
tera.register_tester("odd", testers::odd);
```

Tera has many [built-in tests](./docs/templates.md#built-in-tests) that you can use.

## Global functions
Global functions are Rust code that return a `Result<Value>` from the given params.

Quite often, global functions will need to capture some external variables, such as a `url_for` global function needing
the list of URLs for example. 
To make that work, the type of `GlobalFn` is a boxed closure: `Box<Fn(HashMap<String, Value>) -> Result<Value> + Sync + Send>`.

Here's an example on how to implement a very basic global function:

```rust
fn make_url_for(urls: BTreeMap<String, String>) -> GlobalFn {
    Box::new(move |args| -> Result<Value> {
        match args.get("name") {
            Some(val) => match from_value::<String>(val.clone()) {
                Ok(v) =>  Ok(to_value(urls.get(&v).unwrap()).unwrap()),
                Err(_) => Err("oops".into()),
            },
            None => Err("oops".into()),
        }
    })
}
```
You then need to add it to Tera:

```rust
tera.register_global_function("url_for", make_url_for(urls));
```

And you can now call it from a template:

```jinja2
{{ url_for(name="home") }}
```

Currently global functions can be called in two places in templates:

- variable block: `{{ url_for(name="home") }}`
- for loop container: `{% for i in range(end=5) %}`

Tera comes with some [built-in global functions](./docs/templates.md#built-in-global-functions).

## Assignments
You can assign values to variables during the rendering. 
Assignments in for loops and macros are scoped to their context but
assignments outside of those will be set in the global context.

```jinja2
{% set my_var = "hello" %}
{% set my_var = 1 + 4 %}
{% set my_var = some_var %}
{% set my_var = macros::some_macro() %}
{% set my_var = global_fn() %}
```

If you want to assign a value in the global context while in a forloop, you can use `set_global`:

```jinja2
{% set_global my_var = "hello" %}
{% set_global my_var = 1 + 4 %}
{% set_global my_var = some_var %}
{% set_global my_var = macros::some_macro() %}
{% set_global my_var = global_fn() %}
```

## Comments
To comment out part of the template, wrap it in `{# #}`. Anything in between those tags
will not be rendered.

```jinja2
{# A comment #}
```

## If
Conditionals are fully supported and are identical to the ones in Python.

```jinja2
{% if price < 10 or always_show %}
   Price is {{ price }}.
{% elif price > 1000 and not rich %}
   That's expensive!
{% else %}
    N/A
{% endif %}
```

Undefined variables are considered falsy. This means that you can test for the
presence of a variable in the current context by writing:

```jinja2
{% if my_var %}
    {{ my_var }}
{% else %}
    Sorry, my_var isn't defined.
{% endif %}
```
Every `if` statement has to end with an `endif` tag.

## For

Loop over items in a array:
```jinja2
{% for product in products %}
  {{loop.index}}. {{product.name}}
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
## Include

You can include a template to be rendered using the current context with the `include` tag.

```jinja
{% include "included.html" %}
```

Tera doesn't offer passing a custom context to the `include` tag. 
If you want to do that, use [macros](./docs/templates.md#macros).

## Inheritance

Tera uses the same kind of inheritance as Jinja2 and Django templates: 
you define a base template and extends it in child templates through blocks.
There can be multiple levels of inheritance (i.e. A extends B that extends C).

#### Base template
A base template typically contains the basic document structure as well as 
several `blocks` that can have content.

For example, here's a `base.html` almost copied from the Jinja2 documentation:

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
The only difference with Jinja2 being that the `endblock` tags have to be named.

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

Nested blocks also work in Tera. Consider the following templates:

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

## Macros

Think of macros as functions or components that you can call and return some text.
Macros currently need to be defined in a separate file and imported to be useable.

They are defined as follows:

```jinja2
{% macro input(label, type="text") %}
    <label>
        {{ label }}
        <input type="{{type}}" />
    </label>
{% endmacro hello_world %}
```
As shown in the example above, macro arguments can have a default [literal](./docs/templates.md#literals) value.

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
Macros can be called recursively but there is no limit to recursion so make sure you macro ends.

Here's an example of a recursive macro:

```jinja2
{% macro factorial(n) %}
  {% if n > 1 %}{{ n }} - {{ self::factorial(n=n-1) }}{% else %}1{% endif %}
{% endmacro factorial %}
```

Macros body can contain all normal Tera syntax with the exception of macros definition, `block` and `extends`.

## Raw

Tera will consider all text inside the `raw` block as a string and won't try to
render what's inside. Useful if you have text that contains Tera delimiters.

```jinja2
{% raw %}
  Hello {{ name }}
{% endraw %}
```
would be rendered as `Hello {{ name }}`.

## Whitespace control

Tera comes with easy to use whitespace control: use `{%-` if you want to remove all whitespace
before a statement and `-%}` if you want to remove all whitespace after.

For example, let's look at the following template:

```jinja2
{% set my_var = 2 %}
{{ my_var }}
```

will have the following output:

```html

2
```

If we want to get rid of the empty line, we can write the following:

```jinja2
{% set my_var = 2 -%}
{{ my_var }}
```

## Built-in filters

Tera has the following filters built-in:

### lower
Lowercase a string

### wordcount
Returns number of words in a string

### capitalize
Returns the string with all its character lowercased apart from the first char which is uppercased.

### replace
Takes 2 mandatory string named arguments: `from` and `to`. It will return a string with all instances of 
the `from` string with the `to` string.

Example: `{{ name | replace(from="Robert", to="Bob")}}`

### addslashes
Adds slashes before quotes.

Example: `{{ value | addslashes }}` 

If value is "I'm using Tera", the output will be "I\'m using Tera".

### slugify
Transform a string into ASCII, lowercase it, trim it, converts spaces to hyphens and 
remove all characters that are not numbers, lowercase letters or hyphens.

Example: `{{ value | slugify }}`

If value is "-Hello world! ", the output will be "hello-world".

### title
Capitalizes each word inside a sentence.

Example: `{{ value | title }}`

If value is "foo  bar", the output will be "Foo  Bar".

### trim
Remove leading and trailing whitespace if the variable is a string.

### truncate
Truncates a string to the indicated length. If the string has a smaller length than
the `length` argument, the string is returned as is.

Example: `{{ value | truncate(length=10) }}`

### striptags
Tries to remove HTML tags from input. Does not guarantee well formed output if input is not valid HTML.

Example: `{{ value | striptags}}`

If value is "<b>Joel</b>", the output will be "Joel".

### first
Returns the first element of an array.
If the array is empty, returns empty string.

### last
Returns the last element of an array.
If the array is empty, returns empty string.

### join
Joins an array with a string.

Example: `{{ value| join(sep=" // ") }}`

If value is the array `['a', 'b', 'c']`, the output will be the string "a // b // c".

### length
Returns the length of an array or a string, 0 if the value is not an array.

### reverse
Returns a reversed string or array.

### sort
Sorts an array into ascending order.

The values in the array must be a sortable type:
- numbers are sorted by their numerical value.
- strings are sorted in alphabetical order.
- arrays are sorted by their length.
- bools are sorted as if false=0 and true=1

If you need to sort a list of structs or tuples, use the `attribute`
argument to specify which field to sort by.

Example:

Given `people` is an array of Person

```rust
struct Name(String, String);

struct Person {
    name: Name,
    age: u32
}
```

The `attribute` argument can be used to sort by last name:

```jinja2
{{ people | sort(attribute="name.1") }}
```

or by age:

```jinja2
{{ people | sort(attribute="age") }}
```

### slice
Slice an array by the given `start` and `end` parameter. Both parameters are
optional and omitting them will return the same array.
Use the `start` argument to define where to start (inclusive, default to `0`)
and `end` argument to define where to stop (exclusive, default to the length of the array).
`start` and `end` are 0-indexed.

```jinja2
{% for i in my_arr | slice(end=5) %}
{% for i in my_arr | slice(start=1) %}
{% for i in my_arr | slice(start=1, end=5) %}
```




### urlencode
Percent-encodes a string.

Example: `{{ value | urlencode }}`

If value is `/foo?a=b&c=d`, the output will be `/foo%3Fa%3Db%26c%3Dd`.

Takes an optional argument of characters that shouldn't be percent-encoded (`/` by default). 
So, to encode slashes as well, you can do `{{ value | urlencode(safe="") }}`. 

### pluralize
Returns a suffix if the value is greater or equal than 2. Suffix defaults to `s`

Example: `You have {{ num_messages }} message{{ num_messages|pluralize }}`

If num_messages is 1, the output will be You have 1 message. If num_messages is 2 the output will be You have 2 messages.
You can specify the suffix as an argument that way: `{{ num_messages|pluralize(suffix="es") }}`

### round
Returns a number rounded following the method given. Default method is `common` which will round to the nearest integer.
`ceil` and `floor` are available as alternative methods.
Another optional argument, `precision`, is available to select the precision of the rounding. It defaults to `0`, which will
round to the nearest integer for the given method.

Example: `{{ num | round }} {{ num | round(method="ceil", precision=2) }}`

### filesizeformat
Returns a human-readable file size (i.e. '110 MB') from an integer.

Example: `{{ num | filesizeformat }}`

### date
Parse a timestamp into a date(time) string. Defaults to `YYYY-MM-DD` format.
Time formatting syntax is inspired from strftime and a full reference is available 
on [chrono docs](https://lifthrasiir.github.io/rust-chrono/chrono/format/strftime/index.html).

Example: `{{ ts | date }} {{ ts | date(format="%Y-%m-%d %H:%M") }}`

### escape
Escapes a string's HTML. Specifically, it makes these replacements:

- `&` is converted to `&amp;`
- `<` is converted to `&lt;`
- `>` is converted to `&gt;`
- `"` (double quote) is converted to `&quot;`
- `'` (single quote) is converted to `&#x27;`
- `/` is converted to `&#x27;`
- `` ` `` is converted to `&#96;`

### safe
Mark a variable as safe: HTML will not be escaped anymore.
Currently the position of the safe filter does not matter, e.g.
`{{ content | safe | replace(from="Robert", to="Bob") }}` and `{{ content | replace(from="Robert", to="Bob") | safe }}` will output the same thing.

### get
Access a value from an object when the key is not a Tera identifier.
Example: `{{ sections | get(key="posts/content") }}`

### split
Split a string into an array of strings, separated by a pattern given.
Example: `{{ path | split(pat="/") }}`

### json_encode
Transforms any value into a JSON representation. This filter is better used together with `safe` or when automatic escape is disabled.

Example: `{{ value | safe | json_encode() }}`

It accepts a parameter `pretty` (boolean) to print a formatted JSON instead of a one-liner.

Example: `{{ value | safe | json_encode(pretty=true) }}`

### default
Returns the default value given if the variable evaluated is not present in the context.

Example: `{{ value | default(value=1) }}`


## Built-in tests

Here are the currently built-in tests:

### defined
Returns true if the given variable is defined.

### undefined
Returns true if the given variable is undefined.

### odd
Returns true if the given variable is an odd number.

### even
Returns true if the given variable is an even number.

### string
Returns true if the given variable is a string.

### number
Returns true if the given variable is a number.

### divisibleby
Returns true if the given expression is divisible by the arg given.

Example:
```jinja2
{% if rating is divisibleby(2) %}
    Divisible
{% endif %}
```

### iterable
Returns true if the given variable can be iterated over in Tera (ie is an array/tuple).

### starting\_with
Returns true if the given variable is a string starts with the arg given.

Example:
```jinja2
{% if path is starting_with("x/") %}
    In section x
{% endif %}
```

### ending\_with
Returns true if the given variable is a string ends with the arg given.

### containing
Returns true if the given variable contains the arg given.

The test works on:

- strings: is the arg a substring?
- arrays: is the arg given one of the member of the array?
- maps: is the arg given a key of the map?

Example:
```jinja2
{% if username is containing("xXx") %}
    Bad
{% endif %}
```

## Built-in global functions
Tera comes with some built-in global functions.

### range

Returns an array of integers created using the arguments given. 
There are 3 arguments, all integers:

- `end`: where to stop, mandatory
- `start`: where to start from, defaults to `0`
- `step_by`: with what number do we increment, defaults to `1`
