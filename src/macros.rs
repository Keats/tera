/// Helper macro to get real values out of Value while retaining
/// proper errors in filters
///
/// Takes 4 args:
///
/// - the filter name,
/// - the variable name: use "value" if you are using it on the variable the filter is ran on
/// - the expected type
/// - the actual variable
///
/// ```no_compile
/// let arr = try_get_value!("first", "value", Vec<Value>, value);
/// let val = try_get_value!("pluralize", "suffix", String, val.clone());
/// ```
#[macro_export]
macro_rules! try_get_value {
    ($filter_name:expr, $var_name:expr, $ty:ty, $val:expr) => {{
        match $crate::from_value::<$ty>($val.clone()) {
            Ok(s) => s,
            Err(_) => {
                if $var_name == "value" {
                    return Err($crate::Error::msg(format!(
                        "Filter `{}` was called on an incorrect value: got `{}` but expected a {}",
                        $filter_name, $val, stringify!($ty)
                    )));
                } else {
                    return Err($crate::Error::msg(format!(
                        "Filter `{}` received an incorrect type for arg `{}`: got `{}` but expected a {}",
                        $filter_name, $var_name, $val, stringify!($ty)
                    )));
                }
            }
        }
    }};
}
