/// Filters operating on numbers
use std::collections::HashMap;

use serde_json::value::{Value, to_value};
use errors::TeraResult;


pub fn pluralize(value: Value, args: HashMap<String, Value>) -> TeraResult<Value> {
    let num = try_get_value!("pluralize", "value", f32, value);
    let suffix = match args.get("suffix") {
        Some(val) => try_get_value!("pluralize", "suffix", String, val.clone()),
        None => "s".to_string(),
    };

    if num > 1.0 {
        Ok(to_value(&suffix))
    } else {
        Ok(to_value(&""))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use serde_json::value::to_value;
    use super::*;

    #[test]
    fn test_pluralize_single() {
        let result = pluralize(to_value(1), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(""));
    }

    #[test]
    fn test_pluralize_multiple() {
        let result = pluralize(to_value(2), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("s"));
    }

    #[test]
    fn test_pluralize_multiple_custom_suffix() {
        let mut args = HashMap::new();
        args.insert("suffix".to_string(), to_value("es"));
        let result = pluralize(to_value(2), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("es"));
    }
}
