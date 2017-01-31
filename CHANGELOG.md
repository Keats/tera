# Changelog

## 0.7.0 (unreleased)

### Breaking changes

- `Tera::add_template` -> `Tera::add_raw_template`
- `Tera::add_templates` -> `Tera::add_raw_templates`
- Rendering undefined variables is no longer an error

### Others

- Performance improvement thanks to @clarcharr
- Better error message for `value_render`. Thanks to @SilverWingedSeraph for the report
- Hide `add_raw_template` and `add_raw_templates` from docs, they were meant for internal use
- Exported macros now use the `$crate` variable, which means you don't need to import anything from Tera to have
them working
- Expose AST (not covered by semver), see `lib.rs` and `examples/ast.rs` for information

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
