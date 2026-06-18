use tera::value::ValueKind;
use tera::{Error, Kwargs, State, TeraResult, Value};

/// Putting a huge number for padding could trigger a huge allocation.
const MAX_SPEC_NUM: usize = 100;

/// Formats a value using Rust's std::fmt format specifiers.
///
/// Supports width, alignment, precision, sign, and zero-padding.
/// Does NOT support radix specifiers (x, X, b, o) - only Display formatting.
///
/// # Example
/// ```text
/// {{ 3.14159 | format(spec=".2") }} -> "3.14"
/// {{ 42 | format(spec="05") }} -> "00042"
/// {{ "hi" | format(spec=">5") }} -> "   hi"
/// {{ 42 | format(spec="+") }} -> "+42"
/// ```
pub fn format(val: Value, kwargs: Kwargs, _: &State) -> TeraResult<String> {
    let spec = kwargs.must_get::<&str>("spec")?;
    if spec.contains('$') {
        return Err(Error::message(format!(
            "format spec `{spec}` cannot reference arguments with `$`"
        )));
    }
    for digits in spec.split(|c: char| !c.is_ascii_digit()) {
        if !digits.is_empty() && !digits.parse::<usize>().is_ok_and(|n| n <= MAX_SPEC_NUM) {
            return Err(Error::message(format!(
                "format spec `{spec}` contains a number bigger than the maximum allowed ({MAX_SPEC_NUM})"
            )));
        }
    }
    let fmt_str = format!("{{:{}}}", spec);

    match val.kind() {
        ValueKind::String => {
            let s = val.as_str().unwrap();
            formatx::formatx!(&fmt_str, s)
                .map_err(|e| Error::message(format!("format error: {}", e)))
        }
        ValueKind::I64 | ValueKind::I128 | ValueKind::U64 => {
            let n = val.as_i128().unwrap();
            formatx::formatx!(&fmt_str, n)
                .map_err(|e| Error::message(format!("format error: {}", e)))
        }
        ValueKind::U128 => {
            let n = val.as_u128().unwrap();
            formatx::formatx!(&fmt_str, n)
                .map_err(|e| Error::message(format!("format error: {}", e)))
        }
        ValueKind::F64 => {
            let n = val.as_number().unwrap();
            let f = n.as_float();
            formatx::formatx!(&fmt_str, f)
                .map_err(|e| Error::message(format!("format error: {}", e)))
        }
        ValueKind::Bool => {
            let b = val.as_bool().unwrap();
            formatx::formatx!(&fmt_str, b)
                .map_err(|e| Error::message(format!("format error: {}", e)))
        }
        _ => Err(Error::message(format!(
            "Cannot format value of type {} with format filter",
            val.name()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tera::{Context, Tera};

    fn render(template: &str) -> String {
        let mut tera = Tera::default();
        tera.register_filter("format", format);
        tera.add_raw_template("test", template).unwrap();
        tera.render("test", &Context::new()).unwrap()
    }

    #[test]
    fn test_float_precision() {
        assert_eq!(render("{{ 3.14159 | format(spec='.2') }}"), "3.14");
        assert_eq!(render("{{ 3.14159 | format(spec='.4') }}"), "3.1416");
    }

    #[test]
    fn test_integer_padding() {
        assert_eq!(render("{{ 42 | format(spec='05') }}"), "00042");
        assert_eq!(render("{{ 42 | format(spec='10') }}"), "        42");
    }

    #[test]
    fn test_string_alignment() {
        assert_eq!(render("{{ 'hi' | format(spec='>5') }}"), "   hi");
        assert_eq!(render("{{ 'hi' | format(spec='<5') }}"), "hi   ");
        assert_eq!(render("{{ 'hi' | format(spec='^5') }}"), " hi  ");
    }

    #[test]
    fn test_sign() {
        assert_eq!(render("{{ 42 | format(spec='+') }}"), "+42");
        assert_eq!(render("{{ -42 | format(spec='+') }}"), "-42");
    }

    #[test]
    fn test_bool() {
        assert_eq!(render("{{ true | format(spec='>8') }}"), "    true");
    }

    #[test]
    fn test_combined() {
        assert_eq!(render("{{ 42 | format(spec='>+10') }}"), "       +42");
        assert_eq!(render("{{ 3.14159 | format(spec='>10.2') }}"), "      3.14");
    }

    #[test]
    fn test_huge_width_rejected() {
        let mut tera = Tera::default();
        tera.register_filter("format", format);
        for spec in ["4000000000", ".4000000000", ">99999999999999999999"] {
            tera.add_raw_template("test", &format!("{{{{ 1 | format(spec='{spec}') }}}}"))
                .unwrap();
            let err = tera.render("test", &Context::new()).unwrap_err();
            assert!(err.to_string().contains("maximum allowed"), "{err}");
        }
    }

    #[test]
    fn test_dollar_reference_rejected() {
        let mut tera = Tera::default();
        tera.register_filter("format", format);
        // `0$` would use the value (50000) as the width
        for spec in ["0$", "0$.2", ">0$"] {
            tera.add_raw_template("test", &format!("{{{{ 50000 | format(spec='{spec}') }}}}"))
                .unwrap();
            let err = tera.render("test", &Context::new()).unwrap_err();
            assert!(err.to_string().contains('$'), "{err}");
        }
    }
}
