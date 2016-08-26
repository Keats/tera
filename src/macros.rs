/// Helper macro to get real values out of Value while retaining
/// proper errors
/// Takes 4 args: filter name, variable name (use `value` if it's the value the filter
/// is ran on), the expected type and the actual variable
#[macro_export]
macro_rules! try_get_value {
    ($filter_name:expr, $var_name:expr, $ty:ty, $val:expr) => {{
        let v: $ty = match ::serde_json::value::from_value($val.clone()) {
            Ok(s) => s,
            Err(_) => {
                return Err(::errors::TeraError::FilterIncorrectArgType(
                    $filter_name.to_string(),
                    $var_name.to_string(),
                    $val,
                    stringify!($ty).to_string())
                );
            }
        };
        v
    }};
}
