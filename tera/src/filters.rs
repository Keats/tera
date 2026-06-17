use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fmt::Write;
use std::sync::Arc;

use crate::args::{ArgFromValue, Kwargs};
use crate::errors::{Error, TeraResult};
use crate::utils::escape_html;
use crate::value::number::Number;
use crate::value::{FunctionResult, Key, Map, ValueKind};
use crate::vm::state::State;
use crate::{HashMap, Value};

/// The filter function type definition
pub trait Filter<Arg, Res>: Sync + Send + 'static {
    /// The filter function type definition
    fn call(&self, value: Arg, kwargs: Kwargs, state: &State) -> Res;

    /// Whether the current filter's output should be treated as safe, defaults to `false`
    /// Only needs to be defined if the filter returns a string
    fn is_safe(&self) -> bool {
        false
    }
}

impl<Func, Arg, Res> Filter<Arg, Res> for Func
where
    Func: Fn(Arg, Kwargs, &State) -> Res + Sync + Send + 'static,
    Arg: for<'a> ArgFromValue<'a>,
    Res: FunctionResult,
{
    fn call(&self, value: Arg, kwargs: Kwargs, state: &State) -> Res {
        (self)(value, kwargs, state)
    }
}

type FilterFunc = dyn Fn(&Value, Kwargs, &State) -> TeraResult<Value> + Sync + Send + 'static;

#[derive(Clone)]
pub(crate) struct StoredFilter {
    func: Arc<FilterFunc>,
    is_safe: bool,
}

impl std::fmt::Debug for StoredFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredFilter")
            .field("is_safe", &self.is_safe)
            .finish_non_exhaustive()
    }
}

impl StoredFilter {
    pub fn new<Func, Arg, Res>(f: Func) -> Self
    where
        Func: Filter<Arg, Res> + for<'a> Filter<<Arg as ArgFromValue<'a>>::Output, Res>,
        Arg: for<'a> ArgFromValue<'a>,
        Res: FunctionResult,
    {
        let is_safe = Filter::<Arg, Res>::is_safe(&f);
        let closure = move |arg: &Value, kwargs, state: &State| -> TeraResult<Value> {
            f.call(Arg::from_value(arg)?, kwargs, state).into_result()
        };

        StoredFilter {
            func: Arc::new(closure),
            is_safe,
        }
    }

    pub fn call(&self, arg: &Value, kwargs: Kwargs, state: &State) -> TeraResult<Value> {
        (self.func)(arg, kwargs, state)
    }

    pub fn is_safe(&self) -> bool {
        self.is_safe
    }
}

pub(crate) fn safe(val: Cow<'_, str>, _: Kwargs, _: &State) -> Value {
    Value::safe_string(&val)
}

pub(crate) fn default(val: Value, kwargs: Kwargs, _: &State) -> TeraResult<Value> {
    let default_val = kwargs.must_get::<Value>("value")?;
    let boolean = kwargs.get::<bool>("boolean")?.unwrap_or_default();

    if boolean {
        if val.is_truthy() {
            Ok(val)
        } else {
            Ok(default_val)
        }
    } else {
        match val.kind() {
            ValueKind::Undefined => Ok(default_val),
            _ => Ok(val),
        }
    }
}

pub(crate) fn upper(val: &str, _: Kwargs, _: &State) -> String {
    val.to_uppercase()
}

pub(crate) fn lower(val: &str, _: Kwargs, _: &State) -> String {
    val.to_lowercase()
}

pub(crate) fn wordcount(val: &str, _: Kwargs, _: &State) -> usize {
    val.split_whitespace().count()
}

pub(crate) fn escape(val: &str, _: Kwargs, _: &State) -> String {
    let mut buf = Vec::with_capacity(val.len());
    escape_html(val, &mut buf).unwrap();
    // SAFETY: escape_html only produces valid UTF-8
    unsafe { String::from_utf8_unchecked(buf) }
}

pub(crate) fn escape_xml(val: &str, _: Kwargs, _: &State) -> String {
    let mut output = String::with_capacity(val.len() * 2);
    for c in val.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(c),
        }
    }
    output
}

pub(crate) fn newlines_to_br(val: &str, _: Kwargs, _: &State) -> String {
    val.replace("\r\n", "<br>").replace(['\n', '\r'], "<br>")
}

/// Returns a plural suffix if the value is not equal to ±1, or a singular suffix otherwise.
/// Default singular suffix is "" and default plural suffix is "s".
pub(crate) fn pluralize(val: Value, kwargs: Kwargs, _: &State) -> TeraResult<String> {
    let singular = kwargs.get::<&str>("singular")?.unwrap_or("");
    let plural = kwargs.get::<&str>("plural")?.unwrap_or("s");

    let is_singular = match val.as_i128() {
        Some(n) => n == 1 || n == -1,
        None => {
            return Err(Error::message(format!(
                "pluralize filter requires an integer, got `{}`",
                val.name()
            )));
        }
    };

    Ok(if is_singular { singular } else { plural }.to_string())
}

pub(crate) fn trim(val: &str, kwargs: Kwargs, _: &State) -> TeraResult<String> {
    if let Some(pat) = kwargs.get::<&str>("pat")? {
        Ok(val
            .trim_start_matches(pat)
            .trim_end_matches(pat)
            .to_string())
    } else {
        Ok(val.trim().to_string())
    }
}

pub(crate) fn trim_start(val: &str, kwargs: Kwargs, _: &State) -> TeraResult<String> {
    if let Some(pat) = kwargs.get::<&str>("pat")? {
        Ok(val.trim_start_matches(pat).to_string())
    } else {
        Ok(val.trim_start().to_string())
    }
}

pub(crate) fn trim_end(val: &str, kwargs: Kwargs, _: &State) -> TeraResult<String> {
    if let Some(pat) = kwargs.get::<&str>("pat")? {
        Ok(val.trim_end_matches(pat).to_string())
    } else {
        Ok(val.trim_end().to_string())
    }
}

pub(crate) fn replace(val: &str, kwargs: Kwargs, _: &State) -> TeraResult<String> {
    let from = kwargs.must_get::<&str>("from")?;
    let to = kwargs.must_get::<&str>("to")?;

    Ok(val.replace(from, to))
}

/// Uppercase the first char and lowercase the rest.
pub(crate) fn capitalize(val: &str, _: Kwargs, _: &State) -> String {
    let mut chars = val.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
    }
}

/// Uppercase the first letter of each word
pub(crate) fn title(val: &str, _: Kwargs, _: &State) -> String {
    let mut res = String::with_capacity(val.len());
    let mut capitalize = true;
    for c in val.chars() {
        if c.is_ascii_punctuation() || c.is_whitespace() {
            res.push(c);
            // Special case the apostrophe so that it doesn't mess up the English 's etc
            if c != '\'' {
                capitalize = true;
            }
        } else if capitalize {
            write!(res, "{}", c.to_uppercase()).unwrap();
            capitalize = false;
        } else {
            write!(res, "{}", c.to_lowercase()).unwrap();
        }
    }
    res
}

/// Works on char/graphemes, not bytes.
pub(crate) fn truncate(val: &str, kwargs: Kwargs, _: &State) -> TeraResult<String> {
    let length = kwargs.must_get::<usize>("length")?;
    let end = kwargs.get::<&str>("end")?.unwrap_or("…");

    #[cfg(feature = "unicode")]
    {
        use unicode_segmentation::UnicodeSegmentation;
        let graphemes = val.grapheme_indices(true).collect::<Vec<(usize, &str)>>();
        if length >= graphemes.len() {
            return Ok(val.to_string());
        }
        Ok(val[..graphemes[length].0].to_string() + end)
    }

    #[cfg(not(feature = "unicode"))]
    {
        match val.char_indices().nth(length) {
            Some((byte_idx, _)) => Ok(val[..byte_idx].to_string() + end),
            None => Ok(val.to_string()),
        }
    }
}

/// Return a copy of the string with each line indented by 4 spaces.
/// The first line and blank lines are not indented by default.
/// Max width of 1000 to avoid DOS
pub(crate) fn indent(val: &str, kwargs: Kwargs, _: &State) -> TeraResult<String> {
    let width = kwargs.get::<usize>("width")?.unwrap_or(4).min(1000);
    let indent_first_line = kwargs.get::<bool>("first")?.unwrap_or(false);
    let indent_blank_line = kwargs.get::<bool>("blank")?.unwrap_or(false);

    let indent = " ".repeat(width);
    let mut res = String::with_capacity(val.len() * 2);

    let mut first_line = true;
    for line in val.lines() {
        if first_line {
            if indent_first_line {
                res.push_str(&indent);
            }
            first_line = false
        } else {
            res.push('\n');
            if !line.is_empty() || indent_blank_line {
                res.push_str(&indent);
            }
        }
        res.push_str(line);
    }

    if val.ends_with('\n') {
        res.push('\n');
    }

    Ok(res)
}

pub(crate) fn as_str(val: Value, _: Kwargs, _: &State) -> String {
    format!("{val}")
}

/// Converts a Value into an int. It defaults to a base of `10` but can be changed.
pub(crate) fn int(val: Value, kwargs: Kwargs, _: &State) -> TeraResult<Value> {
    let base = kwargs.get::<u32>("base")?.unwrap_or(10);
    if !(2..=36).contains(&base) {
        return Err(Error::message(format!(
            "int filter `base` must be between 2 and 36, got {base}"
        )));
    }

    let handle_f64 =
        |v: f64| {
            Number::Float(v).as_integer().map(Into::into).ok_or_else(|| {
            Error::message(format!(
                "The float {v} cannot be converted to an int (non-integer or out of i128 range)"
            ))
        })
        };

    match val.kind() {
        ValueKind::String => {
            let s = val.as_str().unwrap().trim();
            let s = match base {
                2 => s.trim_start_matches("0b"),
                8 => s.trim_start_matches("0o"),
                16 => s.trim_start_matches("0x"),
                _ => s,
            };
            match i128::from_str_radix(s, base) {
                Ok(v) => Ok(v.into()),
                Err(_) => {
                    if s.contains('.') {
                        match s.parse::<f64>() {
                            Ok(f) => handle_f64(f),
                            Err(_) => Err(Error::message(format!(
                                "The string `{s}` cannot be converted to an int in base {base}"
                            ))),
                        }
                    } else {
                        Err(Error::message(format!(
                            "The string `{s}` cannot be converted to an int in base {base}"
                        )))
                    }
                }
            }
        }
        ValueKind::U64 => {
            let v = val.as_i128().unwrap() as u64;
            Ok(v.into())
        }
        ValueKind::I64 => {
            let v = val.as_i128().unwrap();
            Ok(v.into())
        }
        ValueKind::I128 => {
            let v = val.as_i128().unwrap();
            Ok(v.into())
        }
        ValueKind::U128 => Ok(val),
        ValueKind::F64 => {
            let v = val.as_f64().unwrap();
            handle_f64(v)
        }
        _ => Err(Error::message(format!(
            "Value of type {} cannot be converted to an int",
            val.name()
        ))),
    }
}

pub(crate) fn float(val: Value, _: Kwargs, _: &State) -> TeraResult<f64> {
    match val.kind() {
        ValueKind::String => {
            let s = val.as_str().unwrap().trim();
            if let Ok(num) = s.parse::<f64>() {
                Ok(num)
            } else {
                Err(Error::message(format!(
                    "The string `{s}` cannot be converted to a float"
                )))
            }
        }
        _ => {
            if let Some(num) = val.as_number() {
                Ok(num.as_float())
            } else {
                Err(Error::message(format!(
                    "Value of type {} cannot be converted to a float",
                    val.name()
                )))
            }
        }
    }
}

pub(crate) fn length(val: Value, _: Kwargs, _: &State) -> TeraResult<usize> {
    match val.len() {
        Some(v) => Ok(v),
        None => Err(Error::message(format!(
            "Value of type {} has no length",
            val.name()
        ))),
    }
}

pub(crate) fn reverse(val: Value, _: Kwargs, _: &State) -> TeraResult<Value> {
    val.reverse()
}

pub(crate) fn split(val: &str, kwargs: Kwargs, _: &State) -> TeraResult<Value> {
    let pat = kwargs.must_get::<&str>("pat")?;
    Ok(val
        .split(pat)
        .map(Into::into)
        .collect::<Vec<Value>>()
        .into())
}

pub(crate) fn abs(val: Value, _: Kwargs, _: &State) -> TeraResult<Value> {
    match val.kind() {
        ValueKind::U64 | ValueKind::U128 => Ok(val),
        ValueKind::F64 => {
            let v = val.as_f64().unwrap();
            Ok(v.abs().into())
        }
        ValueKind::I64 => {
            let v = val.as_i128().unwrap() as i64;
            match v.checked_abs() {
                Some(v) => Ok(v.into()),
                None => Ok((v as i128).abs().into()),
            }
        }
        ValueKind::I128 => {
            let v = val.as_i128().unwrap();
            match v.checked_abs() {
                Some(v) => Ok(v.into()),
                None => Err(Error::message(
                    "Errored while getting absolute value: it is i128::MIN value.".to_string(),
                )),
            }
        }
        _ => Err(Error::message(format!(
            "This filter can only be used on a number, received `{}`.",
            val.name()
        ))),
    }
}

pub(crate) fn round(val: f64, kwargs: Kwargs, _: &State) -> TeraResult<Value> {
    let method = kwargs.get::<&str>("method")?;
    let precision = kwargs.get::<i32>("precision")?.unwrap_or_default();
    let multiplier = if precision == 0 {
        1.0
    } else {
        10.0_f64.powi(precision)
    };

    match method {
        Some("ceil") => Ok(((multiplier * val).ceil() / multiplier).into()),
        Some("floor") => Ok(((multiplier * val).floor() / multiplier).into()),
        None => Ok(((multiplier * val).round() / multiplier).into()),
        Some(m) => Err(Error::message(format!(
            "Invalid argument for `method`: {m}. \
                Only `ceil` and `floor` are allowed. \
                Do not fill this parameter if you want a classic round."
        ))),
    }
}

/// Returns the first element of an array. None if the array is empty
/// and errors if the value is not an array
pub(crate) fn first(val: &[Value], _: Kwargs, _: &State) -> TeraResult<Value> {
    Ok(val.first().cloned().unwrap_or(Value::none()))
}

/// Returns the last element of an array. None if the array is empty
/// and errors if the value is not an array
pub(crate) fn last(val: &[Value], _: Kwargs, _: &State) -> TeraResult<Value> {
    Ok(val.last().cloned().unwrap_or(Value::none()))
}

/// Returns the nth element of an array. None if there isn't an element at that index.
/// and errors if the value is not an array
pub(crate) fn nth(val: &[Value], kwargs: Kwargs, _: &State) -> TeraResult<Value> {
    let n = kwargs.must_get::<usize>("n")?;
    Ok(val.get(n).cloned().unwrap_or(Value::none()))
}

/// Joins the elements
pub(crate) fn join(val: &[Value], kwargs: Kwargs, _: &State) -> TeraResult<String> {
    let sep = kwargs.get::<&str>("sep")?.unwrap_or("");
    Ok(val
        .iter()
        .map(|x| format!("{x}"))
        .collect::<Vec<_>>()
        .join(sep))
}

/// We want to check if the items can actually be sorted, eg be comparable. We allow null
/// to stay though but eg a number and a string in the same vec will raise an error.
fn ensure_comparable<'a>(keys: impl Iterator<Item = &'a Value>) -> TeraResult<()> {
    let mut prev: Option<&Value> = None;
    for key in keys {
        if let Some(prev) = prev {
            let skippable = prev.is_none() || key.is_none();
            if !skippable && prev.partial_cmp(key).is_none() {
                return Err(Error::message(format!(
                    "Cannot sort: `{}` and `{}` are not comparable",
                    prev.name(),
                    key.name()
                )));
            }
        }
        prev = Some(key);
    }

    Ok(())
}

/// Sorts an array. If `attribute` is provided, sorts by that attribute.
pub(crate) fn sort(val: &[Value], kwargs: Kwargs, _: &State) -> TeraResult<Vec<Value>> {
    if val.is_empty() {
        return Ok(Vec::new());
    }

    if let Some(attribute) = kwargs.get::<&str>("attribute")? {
        let mut decorated = Vec::with_capacity(val.len());
        for v in val {
            let key = v.get_from_path(attribute);
            if key.is_undefined() {
                return Err(Error::message(format!(
                    "Value {v} does not have an attribute after following path: {attribute}"
                )));
            }
            decorated.push((key, v));
        }
        decorated.sort_by(|(a, _), (b, _)| a.cmp(b));
        ensure_comparable(decorated.iter().map(|(k, _)| k))?;
        Ok(decorated.into_iter().map(|(_, v)| v.clone()).collect())
    } else {
        let mut out = val.to_vec();
        // We sort with Ord::cmp because we have our own custom impl that the default sorting will
        // disagree with
        #[allow(clippy::unnecessary_sort_by)]
        out.sort_by(|a, b| a.cmp(b));
        ensure_comparable(out.iter())?;
        Ok(out)
    }
}

pub(crate) fn unique(val: &[Value], _: Kwargs, _: &State) -> Vec<Value> {
    if val.is_empty() {
        return Vec::new();
    }

    let mut seen = BTreeSet::new();
    let mut res = Vec::with_capacity(val.len());

    for v in val {
        if !seen.contains(v) {
            seen.insert(v.clone());
            res.push(v.clone());
        }
    }

    res
}

/// Map retrieves an attribute from a list of objects and/or applies a filter to each element.
/// - `attribute`: specifies what attribute to retrieve from each element
/// - `filter`: specifies a filter to apply to each element (or to the extracted attribute)
/// - `args`: optional map of arguments to pass to the filter
///
/// At least one of `attribute` or `filter` must be provided.
/// If both are provided, the attribute is extracted first, then the filter is applied.
pub(crate) fn map(val: &[Value], kwargs: Kwargs, state: &State) -> TeraResult<Vec<Value>> {
    if val.is_empty() {
        return Ok(Vec::new());
    }

    let filter_name = kwargs.get::<&str>("filter")?;
    let attribute = kwargs.get::<&str>("attribute")?;

    // Must have at least one of filter or attribute
    if filter_name.is_none() && attribute.is_none() {
        return Err(Error::message(
            "map filter requires either `filter` or `attribute` argument",
        ));
    }

    // Prepare filter kwargs if filter is specified
    let filter_kwargs = if filter_name.is_some() {
        let args_map = kwargs
            .get::<Value>("args")?
            .and_then(|v| v.into_map())
            .map(Arc::new)
            .unwrap_or_else(|| Arc::new(Map::new()));
        Some(Kwargs::new(args_map))
    } else {
        None
    };

    let mut res = Vec::with_capacity(val.len());
    for v in val {
        // Step 1: Extract attribute if specified
        let extracted = if let Some(attr) = attribute {
            match v.get_from_path(attr) {
                x if x.is_undefined() => {
                    return Err(Error::message(format!(
                        "Value {v} does not have an attribute at path: {attr}"
                    )));
                }
                x => x,
            }
        } else {
            v.clone()
        };

        // Step 2: Apply filter if specified
        let final_val = if let (Some(name), Some(f_kwargs)) = (filter_name, &filter_kwargs) {
            state.call_filter(name, &extracted, f_kwargs.clone())?
        } else {
            extracted
        };

        res.push(final_val);
    }
    Ok(res)
}

pub(crate) fn values(val: &Map, _: Kwargs, _: &State) -> TeraResult<Vec<Value>> {
    Ok(val.values().cloned().collect())
}

pub(crate) fn keys(val: &Map, _: Kwargs, _: &State) -> TeraResult<Vec<Value>> {
    Ok(val.keys().map(|k| k.clone().into()).collect())
}

pub(crate) fn pairs(val: &Map, _: Kwargs, _: &State) -> TeraResult<Vec<Value>> {
    Ok(val
        .iter()
        .map(|(k, v)| Value::from(vec![Value::from(k.clone()), v.clone()]))
        .collect())
}

pub(crate) fn get(val: &Map, kwargs: Kwargs, _: &State) -> TeraResult<Value> {
    let key = kwargs.must_get::<&str>("key")?;
    let default = kwargs.get::<Value>("default")?;
    if let Some(val_found) = val.get(&Key::Str(key)) {
        Ok(val_found.clone())
    } else if let Some(d) = default {
        Ok(d)
    } else {
        Err(Error::message(format!(
            "Map does not have a key {key} and no default values were defined"
        )))
    }
}

pub(crate) fn filter(val: &[Value], kwargs: Kwargs, _: &State) -> TeraResult<Vec<Value>> {
    if val.is_empty() {
        return Ok(Vec::new());
    }
    let attribute = kwargs.must_get::<&str>("attribute")?;
    let value = kwargs.get::<Value>("value")?.unwrap_or(Value::none());
    let mut res = Vec::with_capacity(val.len());

    for v in val {
        match v.get_from_path(attribute) {
            x if x.is_undefined() => {
                return Err(Error::message(format!(
                    "Value {v} does not have an attribute after following path: {attribute}"
                )));
            }
            x => {
                if x == value {
                    res.push(v.clone())
                }
            }
        }
    }

    Ok(res)
}

pub(crate) fn group_by(val: &[Value], kwargs: Kwargs, _: &State) -> TeraResult<Map> {
    if val.is_empty() {
        return Ok(Map::new());
    }

    let attribute = kwargs.must_get::<&str>("attribute")?;
    let mut grouped: HashMap<Key, Vec<Value>> = HashMap::new();
    for v in val {
        match v.get_from_path(attribute) {
            x if x.is_undefined() => {
                return Err(Error::message(format!(
                    "Value {v} does not have an attribute after following path: {attribute}"
                )));
            }
            x if x.is_none() => (),
            x => {
                let key = x.as_key()?;
                if let Some(arr) = grouped.get_mut(&key) {
                    arr.push(v.clone());
                } else {
                    grouped.insert(key, vec![v.clone()]);
                }
            }
        }
    }

    Ok(grouped.into_iter().map(|(k, v)| (k, v.into())).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Context;
    use crate::value::Map;

    #[test]
    fn test_title() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        let tests = vec![
            ("foo bar", "Foo Bar"),
            ("foo\tbar", "Foo\tBar"),
            ("foo  bar", "Foo  Bar"),
            ("f bar f", "F Bar F"),
            ("foo-bar", "Foo-Bar"),
            ("FOO\tBAR", "Foo\tBar"),
            ("foo (bar)", "Foo (Bar)"),
            ("foo (bar) ", "Foo (Bar) "),
            ("foo {bar}", "Foo {Bar}"),
            ("foo [bar]", "Foo [Bar]"),
            ("foo <bar>", "Foo <Bar>"),
            ("  foo  bar", "  Foo  Bar"),
            ("\tfoo\tbar\t", "\tFoo\tBar\t"),
            ("foo bar ", "Foo Bar "),
            ("foo bar\t", "Foo Bar\t"),
            ("foo's bar", "Foo's Bar"),
        ];
        for (input, expected) in tests {
            assert_eq!(title(input, Kwargs::default(), &state), expected);
        }
    }

    #[test]
    fn test_str() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        assert_eq!(as_str((2.1).into(), Kwargs::default(), &state), "2.1");
        assert_eq!(as_str(2.into(), Kwargs::default(), &state), "2");
        assert_eq!(as_str(true.into(), Kwargs::default(), &state), "true");
        assert_eq!(
            as_str(vec![1, 2, 3].into(), Kwargs::default(), &state),
            "[1, 2, 3]"
        );
        let mut map = Map::new();
        map.insert("hello".into(), "world".into());
        map.insert("other".into(), 2.into());
        assert_eq!(
            as_str(map.into(), Kwargs::default(), &state),
            r#"{"hello": "world", "other": 2}"#
        );
    }

    #[test]
    fn test_int() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        // String to int
        let tests: Vec<(&str, i64)> = vec![
            ("0", 0),
            ("-5", -5),
            ("9223372036854775807", i64::MAX),
            ("1.00", 1),
        ];
        for (input, expected) in tests {
            assert_eq!(
                int(input.into(), Kwargs::default(), &state).unwrap(),
                expected.into()
            );
        }

        assert_eq!(
            int("0b1010".into(), Kwargs::from([("base", 2.into())]), &state).unwrap(),
            10.into()
        );

        // We don't do anything in that case
        assert_eq!(
            int((-5_i128).into(), Kwargs::default(), &state).unwrap(),
            (-5_i128).into()
        );

        // Can't convert without truncating
        assert!(int(1.12.into(), Kwargs::default(), &state).is_err());

        // Doesn't make sense
        assert!(int("hello".into(), Kwargs::default(), &state).is_err());
        assert!(int(vec![1, 2].into(), Kwargs::default(), &state).is_err());
    }

    #[test]
    fn test_float() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        assert_eq!(float("1".into(), Kwargs::default(), &state).unwrap(), 1.0);
        assert_eq!(
            float("3.16".into(), Kwargs::default(), &state).unwrap(),
            3.16
        );
        assert_eq!(float(1.into(), Kwargs::default(), &state).unwrap(), 1.0);
        // noop
        assert_eq!(float(1.12.into(), Kwargs::default(), &state).unwrap(), 1.12);
        // Doesn't make sense
        assert!(float("hello".into(), Kwargs::default(), &state).is_err());
        assert!(float(vec![1, 2].into(), Kwargs::default(), &state).is_err());
    }

    #[test]
    fn test_escape_xml() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        let tests = vec![
            (r"hey-&-ho", "hey-&amp;-ho"),
            (r"hey-'-ho", "hey-&apos;-ho"),
            (r"hey-&'-ho", "hey-&amp;&apos;-ho"),
            (r#"hey-&'"-ho"#, "hey-&amp;&apos;&quot;-ho"),
            (r#"hey-&'"<-ho"#, "hey-&amp;&apos;&quot;&lt;-ho"),
            (r#"hey-&'"<>-ho"#, "hey-&amp;&apos;&quot;&lt;&gt;-ho"),
        ];
        for (input, expected) in tests {
            assert_eq!(escape_xml(input, Kwargs::default(), &state), expected);
        }
    }

    #[test]
    fn test_abs() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        assert_eq!(abs(1.into(), Kwargs::default(), &state).unwrap(), 1.into());
        assert_eq!(
            abs((-1i64).into(), Kwargs::default(), &state).unwrap(),
            1.into()
        );
        assert_eq!(
            abs((-1.0).into(), Kwargs::default(), &state).unwrap(),
            (1.0).into()
        );
        assert!(abs(i128::MIN.into(), Kwargs::default(), &state).is_err());
        assert!(abs("hello".into(), Kwargs::default(), &state).is_err());
    }

    #[test]
    fn test_round() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        assert_eq!(round(2.1, Kwargs::default(), &state).unwrap(), 2.into());

        assert_eq!(
            round(2.1, Kwargs::from([("method", "ceil".into())]), &state).unwrap(),
            3.into()
        );

        assert_eq!(
            round(2.9, Kwargs::from([("method", "floor".into())]), &state).unwrap(),
            2.into()
        );

        assert_eq!(
            round(2.245, Kwargs::from([("precision", 2.into())]), &state).unwrap(),
            (2.25).into()
        );
    }

    #[cfg(feature = "unicode")]
    #[test]
    fn can_truncate_graphemes() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        let inputs = vec![("日本語", 2, "日本…"), ("👨‍👩‍👧‍👦 family", 5, "👨‍👩‍👧‍👦 fam…")];

        for (input, length, expected) in inputs {
            let out = truncate(input, Kwargs::from([("length", length.into())]), &state).unwrap();
            assert_eq!(out, expected);
        }
    }

    #[cfg(not(feature = "unicode"))]
    #[test]
    fn truncate_splits_on_char_boundary() {
        let ctx = Context::new();
        let state = State::new(&ctx);
        let inputs = [("😀test", 1, "😀…"), ("日本語hello", 3, "日本語…")];

        for (input, length, expected) in inputs {
            let out = truncate(input, Kwargs::from([("length", length.into())]), &state).unwrap();
            assert_eq!(out, expected);
        }
    }
}
