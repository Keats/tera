# Changelog

## 0.10.6 (2017-05-23)

- Fix not being able to call global functions without arguments
- Fix multiple inheritance not rendering blocks as expected for nested blocks
- Allow filters on key/value for loop containers

## 0.10.5 (2017-05-13)

- Fix bug with `{% set %}` in forloops

## 0.10.4 (2017-05-09)

- Add `Send` to `GlobalFn` return type

## 0.10.3 (2017-05-09)

- Add global functions, see README
- Add set tag, see README
- Add get filter

## 0.10.2 (2017-05-03)

- Fix bug with section filter swallowing all content after the end tag
- Allow whitespace in function args

## 0.10.1 (2017-04-25)

- Fix bug with variable in loop using starting with the container name (#165)
- Allow whitespace in macros/filters params

## 0.10.0 (2017-04-21)

### Breaking changes
- Update Serde to 1.0.0

### Others
- Fix date filter converting everything to UTC
- Fix panic when using filters on forloop container

## 0.9.0 (2017-04-05)

### Breaking changes
- Fix bug in Windows where the glob path was not removed correctly

### Others
- `Tera::extend` now also copy filters and testers


## 0.8.1 (2017-03-15)

- Macro rendering perf improved and general code cleanup thanks to @Peternator7
- Fix bug in parser with floats
- Make `date` filter work with string input in `YYYY-MM-DD` format
- Big parsing improvement (~20-40%) for projects with macros and inheritance
- Add `Tera::extend` to extend another instance of Tera
- Add `Tera::full_reload` that will re-run the glob and parse all templates found.
- Make `Tera::add_raw_template{s}` and `Tera::add_template_file{s}` part of the public API
- Fix location in error message when erroring in a child template


## 0.8.0 (2017-03-03)

### Breaking changes
- Remove `value_render` and `value_one_off`, you can now use `render` and `one_off`
for both values and context

### Others
- Speed improvements on both parsing and rendering (~20-40% faster)
- Better error message on variable lookup failure in loops
- Can now iterate on maps/struct using the `{% for key, val in my_object %}` construct


## 0.7.2 (2017-02-18)

- Update chrono version
- Make variable block like `{{ "hey" }}` render correctly

## 0.7.1 (2017-02-05)

- Support filter sections
- Fix path prefix trimming on Windows

## 0.7.0 (2017-02-01)

### Breaking changes

- `Tera::add_template` -> `Tera::add_raw_template`
- `Tera::add_templates` -> `Tera::add_raw_templates`

### Others

- Performance improvement thanks to @clarcharr
- Better error message for `value_render`. Thanks to @SilverWingedSeraph for the report
- Hide `add_raw_template` and `add_raw_templates` from docs, they were meant for internal use
- Exported macros now use the `$crate` variable, which means you don't need to import anything from Tera to have
them working
- Expose AST (not covered by semver)
- Add a `Context::extend` method to merge a context object into another one

## 0.6.2 (2017-01-08)

- Performance improvements thanks to @wdv4758h
- Correctly register `date` filter and make it work on a RFC3339 string as well thanks to @philwhineray

## 0.6.1 (2016-12-28)

- Added `Tera::value_one_off` to parse and render a single template using a
Json value as context

## 0.6.0 (2016-12-26)

### BREAKING CHANGES
- `not` is now a Tera keyword

### Others
- Added `#![deny(missing_docs)]` to the crate
- Added `Tera::one_off` to parse and render a single template
- Added `not` operator in conditions to mean falsiness (equivalent to `!` in Rust)
- Remove specific error message when using `||` or `&&`
- Improved performances for parsing and rendering (~5-20%)
- Added `precision` arg to `round` filter
- Added `date` filter to format a timestamp to a date(time) string

## 0.5.0 (2016-12-19)

A few breaking changes in this one

### BREAKING CHANGES
- Tera no longer panics when parsing templates, it returns an error instead
- Tester fn signature changes from `fn(&str, Option<Value>, Vec<Value>) -> Result<bool>` to `fn(Option<Value>, Vec<Value>) -> Result<bool>`
- Rename `TeraResult` export to `Result`

### Others
- Stabilized `Tera::add_template` and `Tera::add_templates`
- Added `compile_templates!` macro to try to compile all templates and, in case of errors,
print them and exit the process
- Much improved error messages
- Add a magical variable `__tera_context` that will pretty print the current context
- More documentation inside the crate itself
- Actually register the `filesizeformat`, `slugify`, `addslashes`, good thing no one noticed
- Add `divisibleby` and `iterable` test
- Made `try_get_value!` macro work outside of Tera

## 0.4.1 (2016/12/07)

- Remove println! left behind
- Fix macros not being found in child templates
- Export `Value` and `to_value` (currently from serde-json)

## 0.4.0 (2016/12/02)
- Add macros
- Add `filesizeformat` filter
- Add autoescape
- Add multiple level inheritance
- Add nested blocks
- Add `{{ super() }}`

Thanks to @SergioBenitez and @yonran for the help!


## 0.3.1 (2016/10/11)
- Fix regression when using variables in forloops + add test for it

## 0.3.0 (2016/10/11)

- Change signature of tests functions (BREAKING CHANGE)
- Add more tests: `undefined`, `odd`, `even`, `number` and `string`
- Add `include` directive to include another file
- Indexed array/tuple access using the `.x` where `x` is an integer

Thanks to @SergioBenitez and @andrelmartins for the contributions!


## 0.2.0 (2016/09/27)

- Added filters, see README for current list
- Added tests, only `defined` for now

Thanks to @SergioBenitez, @orhanbalci, @foophoof and @Peternator7 for the 
contribution!

## 0.1.3 (2016/08/14)
- Completely new parser
- Expose TeraError
