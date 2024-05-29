# Changelog

## 1.20.0 (2024-05-27)

- Support parenthesis in if statemetns
- Allow escaped newline and tab join separator

## 1.19.1 (2023-09-03)

- Minimum supported Rust version (MSRV) is now 1.63 due to transitive dependencies.
- Update crates.io metadata for the website

## 1.19.0 (2023-05-31)

- Revert change to glob path to not error if the path doesn't exist
- Allow macro definition in renderable template

## 1.18.1 (2023-03-15)

- Fix panic on invalid globs to Tera::new

## 1.18.0 (2023-03-08)

- Add `abs` filter
- Add `indent` filter
- Deprecate `get_json_pointer` in favour of `dotted_pointer`, a faster alternative
- Always canonicalize glob paths passed to Tera to workaround a globwalk bug
- Handle apostrophes in title case filter
- Some performance improvement

## 1.17.1 (2022-09-19)

- Make `get_random` use isize instead of i32 and bad error message
- Fix variables lookup when the evaluated key has a `.` or quotes
- Fix changed output of f64 from serde_json 1.0.85

## 1.17.0 (2022-08-14)

- Fix bug where operands in `in` operation were escaped before comparison
- Force chrono dep to be 0.4.20 minimum
- Better support for parenthesis in expression

## 1.16.0 (2022-06-10)

- Add a feature just for the urlencode builtin
- Fix bug in slice filter if start >= end
- Allow supplying a timezone to a timestamp for the date filter

## 1.15.0 (2021-11-03)

- Add `default` parameter to `get` filter
- `Tera::extend` now also copies over function
- Remove the new Context-local Tera function support, it was an accidental breaking change and will be added in v2 in some ways
instead


## 1.14.0 (2021-11-01) - YANKED as it added a generic to Context, a breaking change

- Ensure `Context` stays valid in Sync+Send, fixing an issue introduced in 1.13. 1.113 will be yanked.

## 1.13.0 (2021-10-17) - YANKED as it made Context not Send+Sync

- Add `default` parameter to `get` filter
- `Tera::extend` now also copies over function
- Add Context-local Tera functions

## 1.12.1 (2021-07-13)

- Remove unused feature of chrono
- Remove unwanted bloat in crate from accidental WASM files

## 1.12.0 (2021-06-29)

- Add `spaceless` filter from Django

## 1.11.0 (2021-06-14)

- Allow iterating on strings with `for`

## 1.10.0 (2021-05-21)

- Add `Tera::get_template_names`

## 1.9.0 (2021-05-16)

- Add `Context::remove`

## 1.8.0 (2021-04-21)

- Add `linebreaksbr` filter from Django
- Allow dots in context object key names

## 1.7.1 (2021-04-12)

- Fix parsing of filter arguments separated by whitespaces

## 1.7.0 (2021-03-07)

- Allow rendering to `std::io::Write`
- Follow symlinks in glob
- Allow including lists of templates
- Comment tags can now use whitespace control

## 1.6.1 (2020-12-29)

- Fix date filter sometimes panicking with some format input

## 1.6.0 (2020-12-19)

- Allow multiline function kwargs with trailing comma
- Add `Context::try_insert`

## 1.5.0 (2020-08-10)

- Add the concept of safe functions and filters
- Allow negative index on `slice` filter

## 1.4.0 (2020-07-24)

- Add `Context::get` and `Context::contains_key`

## 1.3.1 (2020-06-09)

- Fix `raw` tag swallowing all whitespace at beginning and end
- Make batch template sources generic
- Automatically add function/test/filter function name to their error message

## 1.3.0 (2020-05-16)

- Add a `urlencode_strict` filter
- Add more array literals feature in templates
- Make `filter` filter value argument optional

## 1.2.0 (2020-03-29)

- Add `trim_start`, `trim_end`, `trim_start_matches` and `trim_end_matches` filters
- Allow blocks in filter sections

## 1.1.0 (2020-03-08)

- Add Tera::render_str, like Tera::one_off but can use an existing Tera instance

## 1.0.2 (2020-01-13)

- Length filter now errors for things other than array, objects and strings. The fact that it was returning 0 before
for other types was something that should have been fixed before 1.0 but got forgotten and was considered a bug.


## 1.0.1 (2019-12-18)

- Fix filter sections not keeping whitespaces
- The filesizeformat filter now takes a usize instead of a i64: no changes to behaviour

## 1.0.0 (2019-12-07)

### Breaking changes

- Now requires Rust 1.34
- Removed error-chain errors and added rich Error enum instead
- Filter, Tester and Function are now traits and now take borrowed values instead of owned
- Updated for 2018 edition
- Require macros import to be at the top of the files along with `extends` as it is fairly cheap and
the code already only really look there.
- Enforce spacing in tags at the parser, before `ifsomething` was considered ok
- Pluralize filter now uses `singular` and `plural` arguments instead of `suffix`
- Add a test for checking whether a variable is an object
- Escaping now happens before inserting the final result of an expression: no need anymore to add `| safe` everywhere,
only at the last position
- Remove `safe` argument of the urlencode filter, `/` is still escaped by default
- The `compile_templates!` macro has been removed

### Others

- Tests can now use `value is not defined` order for negation (https://github.com/Keats/tera/issues/308)
- Add `nth` filter to get the nth value in an array
- You can now use glob patterns in `Tera::new`
- `default` filter now works on Null values
- Literal numbers in template overflowing i64/f64 will now be an error instead of panicking
- Allow arrays as test arguments
- Add the `in` operator to check if a left operand is contained in a right one. Also supports negation as `not in`
- Add `Context::from_value` to instantiate a `Context` from a serde_json `Value`
- Add `Context::from_serialize` to instantiate a `Context` from something that impl `Serialize`
- Make tests helper fns `number_args_allowed`, `value_defined` and `extract_string` public
- Add `else` clause to for loops
- Filters are now evaluated when checking if/elif conditions
- Allow `{{-` and `-}}` for whitespace management
- Add `xml_escape` filter
- Grave accent is no longer escaped in HTML, it is not really needed anymore
- Add a `builtins` default feature that gate all filters/functions requiring additional dependencies
- Add `unique` and `map` filter
- Add a `timezone` attribute to the `date` filter
- Add a `get_random` function to get a random number in a range
- Add a `get_env` function to get the value of an environment variable

## 0.11.20 (2018-11-14)

- Fix bugs in `filter` and `get` filters

## 0.11.19 (2018-10-31)

- Allow function calls in math expressions
- Allow string concatenation to start with a number
- Allow function calls in string concatenations
- Add a `concat` filter to concat arrays or push an element to an array

## 0.11.18 (2018-10-16)

- Allow concatenation of strings and numbers

## 0.11.17 (2018-10-09)

- Clear local context on each forloop iteration
- Fix variable lookup with `.` that was completely wrong
- Now requires Rust 1.26 because of some dependencies update

## 0.11.16 (2018-09-12)

- Fix `set`/`set_global` not working correctly in macros
- Deprecate `register_global_function` for `register_function`

## 0.11.15 (2018-09-09)

- Remove invalid `unreachable!` call causing panic in some combination or for loop and specific filters
- Fix macros loading in parent templates and using them in child ones
- Fix macros loading other macros not working when called in inheritance
- Mark `Context::add` as deprecated and do not display it in the docs anymore (aka TIL the `deprecated` attribute)
- Fix `__tera_context` not getting all the available context (`set`, `forloop` etc)
- Better error message when variable indexing fails

## 0.11.14 (2018-09-02)

- Remove stray println

## 0.11.13 (2018-09-02)

- Add `as_str` filter
- Way fewer allocations and significant speedup (2-5x) for templates with large objects/loops
- Checks that all macro files are accounted for at compile time and errors if it's not the case

## 0.11.12 (2018-08-04)

- `filter` filter was not properly registered (╯°□°）╯︵ ┻━┻

## 0.11.11 (2018-08-01)

- `truncate` filter now works correctly on multichar graphemes

## 0.11.10 (2018-08-01)

- Add a `throw` global function to fail rendering from inside a template

## 0.11.9 (2018-07-16)

- Add a `matching` tester
- Register `now` global function so it is available
- Update `error-chain`

## 0.11.8 (2018-06-20)

- Add `True` and `False` as boolean values to match Python
- Allow user to define their own escape function, if you want to generate JSON for example
- Add `end` argument to the `truncate` filter to override the default ellipsis
- Add a `group_by` filter
- Add a `filter` filter
- Add the `~` operator to concatenate strings
- Add a `now` global function to get local and UTC datetimes
- Add feature to enable the `preserve_order` feature of serde_json
- Less confusing behaviour with math arithmetics

## 0.11.7 (2018-04-24)

- Add array literal instantiation from inside Tera for set, set_global, kwargs
and for loop container
- Fix panic on truncate filter

## 0.11.6 (2018-03-25)

- Add `break` and `continue` to forloops
- Fix strings delimited by single quote and backtick not removing the delimiters


## 0.11.5 (2018-03-01)

- Re-export `serde_json::Number` as well

## 0.11.4 (2018-02-28)

- Re-export `serde_json::Map` as well
- You can now access inside a variable using index notation: `{{ arr[0] }}`, `{{ arr[idx] }}` etc
thanks to @bootandy
- Add `Context::insert` identical to `Context::add` to mirror Rust HashMap/BTreeMap syntax


## 0.11.3 (2018-02-15)

- Add a `slice` filter for arrays
- Fix macro files importing other macro files not loading properly
- Fix forloop container being allowed logic expressions
- Much improved parsing error messages

## 0.11.2 (2018-02-01)

- Fix regression when including templates that import macros
- Fix `pluralize` filter for real this time!

## 0.11.1 (2018-01-25)

- Fix regression with expressions in comparisons

## 0.11.0 (2018-01-22)

### Breaking changes

- Tests parentheses are now mandatory if there are arguments (`divisibleby 2` -> `divisibleby(2)`)
- Tests can be only used on variables now, not on expressions
- Escaping happens immediately now instead of waiting for the filters to be called, unless `safe` is first.
If you want the old behaviour you will need to start the a chain of filters with `| safe` as the first one

### Others

- Tests, global functions calls and macro calls are now expressions and can be combined like so: `if x is divisibleby(2) and x > 10`
- Add default arguments for macro arguments
- Add whitespace management similar to Liquid and Jinja2
- Add parentheses to expressions to remove ambiguities
- Block & macro end tag name are no longer mandatory and it doesn't error on mismatched names between
the start and end tag anymore
- Filters can now be applied to expressions
- Add modulo operator `%` for math expressions
- Allow comment tags before the extend tag
- Make `NaiveDateTime` work with the `date` filter
- `pluralize` filter now returns the plural suffix for 0 thing as it's apparently what English does
- Add a `set_global` tag that allows you to set something in the global context: meant to be used in forloops where
the normal `set` would put the value into the loop context
- Add `starting_with`, `ending_with` and `containing` tests
- Add `json_encode`, `default` and `sort` filters
- Strings can now also be contained in backticks and single quotes in templates

## 0.10.10 (2017-08-24)

- Add `Tera::parse` for some niche use-cases

## 0.10.9 (2017-08-02)

- Handle path to templates starting with "./"
- Fix loop and macro context overlaps
- Fix variables being escaped when given to `set` or as arguments to filters/macros/global fns

## 0.10.8 (2017-06-24)

- Update chrono

## 0.10.7 (2017-06-16)

- Fix not being able to use variables starting with `or`, `and` and `not`
- Fix `<=` and `>=` not being recognised properly
- Fix if/elif conditions falling through: only the first valid one will be rendered
- Handle NaN results in `{% set %}` instead of panicking
- Allow math node on if/elif conditions & fix f64 truthiness

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
