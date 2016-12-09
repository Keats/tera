/// Helper macro to get real values out of Value while retaining
/// proper errors
/// Takes 4 args: filter name, variable name (use `value` if it's the value the filter
/// is ran on), the expected type and the actual variable
#[macro_export]
macro_rules! try_get_value {
    ($filter_name:expr, $var_name:expr, $ty:ty, $val:expr) => {{
        match ::serde_json::value::from_value::<$ty>($val.clone()) {
            Ok(s) => s,
            Err(_) => {
                bail!(
                    "Filter `{}` received an incorrect type for arg `{}`: got `{:?}` but expected a {}",
                    $filter_name, $var_name, $val, stringify!($ty)
                );
            }
        }
    }};
}

/// Simple macro to compile templates
/// If it fails, it prints the errors and exit the process
#[macro_export]
macro_rules! compile_templates {
    ($glob:expr) => {{
        match Tera::new($glob) {
            Ok(t) => t,
            Err(e) => {
                println!("Error: {}", e);
                for e in e.iter().skip(1) {
                    println!("Reason: {}", e);
                }
                ::std::process::exit(1);
            }
        }
    }};
}
