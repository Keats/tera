# Changelog

## 0.5.0 (Unreleased)

A few breaking changes in this one

### Breaking changes
- Tera no longer panics when parsing templates, it returns an error instead
- Tester fn signature changes from `fn(&str, Option<Value>, Vec<Value>) -> Result<bool>` to `fn(Option<Value>, Vec<Value>) -> Result<bool>`
- Rename `TeraResult` export to `Result`
- Remove `TeraError` export
``
### Others
- Stabilized `Tera::add_template` and `Tera::add_templates`
- Added `compile_templates!` macro  to try to compile all templates and, in case of errors,
print them and exit the process
- Much improved error messages

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
