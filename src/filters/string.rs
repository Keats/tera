/// Filters operating on string
use std::collections::HashMap;

use serde_json::value::{Value, to_value};

use errors::TeraResult;


/// Convert a value to uppercase.
pub fn upper(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let s = try_get_value!("upper", "value", String, value);

    Ok(to_value(&s.to_uppercase()))
}


/// Strip leading and trailing whitespace.
pub fn trim(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let s = try_get_value!("trim", "value", String, value);

    Ok(to_value(&s.trim()))
}

/// Truncates a string to the indicated length
pub fn truncate(value: Value, args: HashMap<String, Value>) -> TeraResult<Value> {
    let s = try_get_value!("truncate", "value", String, value);
    let length = match args.get("length") {
        Some(l) => try_get_value!("truncate", "length", usize, l.clone()),
        None => 255
    };

    // Nothing to truncate?
    if length > s.len() {
        return Ok(to_value(&s));
    }


    let result = s[.. s.char_indices().nth(length).unwrap().0].to_string() + "…";
    Ok(to_value(&result))
}

/// Convert a value to lowercase.
pub fn lower(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let s = try_get_value!("lower", "value", String, value);

    Ok(to_value(&s.to_lowercase()))
}

/// Gets the number of words in a string.
pub fn wordcount(value: Value, _: HashMap<String, Value>) -> TeraResult<Value> {
    let s = try_get_value!("wordcount", "value", String, value);

    Ok(to_value(&s.split_whitespace().count()))
}

/// Replaces given number of substrings with  new ones. If count is not given replaces all
/// occurrences
pub fn replace(value: Value, args: HashMap<String, Value>) -> TeraResult<Value> {
    let mut s = try_get_value!("replace", "value", String, value);

    let count = match args.get("count") {
        Some(c) => try_get_value!("replace", "count", i32, c.clone()),
        None => -1
    };

    let old_chunk = match args.get("old") {
        Some(old) => try_get_value!("replace", "old", String, old.clone()),
        None => String::new() 
    };
    
    let new_chunk = match args.get("new") {
        Some(new) => try_get_value!("replace", "new", String, new.clone()),
        None => String::new()
    };

    // replace all 
    if count == -1 {
        s = s.replace(&old_chunk, &new_chunk);
        Ok(to_value(&s))
    } 
    // replace with count
    else {
        let indices : Vec<_> =  s.match_indices(&old_chunk).collect();
        let mut index_counter : usize = 0;
        let replaced = s.char_indices().fold(String::new(), |acc, x | {
            if (index_counter == 0 && x.0 < indices[index_counter].0) || 
               (index_counter > 0 && x.0 < indices[index_counter].0 && x.0 >= indices[index_counter-1].0 + old_chunk.len()) || 
                index_counter >= count as usize {
                let mut result = acc.clone();
                result.push(x.1);
                result
            }
           else if x.0 == indices[index_counter].0 {
                index_counter += 1;
                let mut result  = acc.clone();
                result.push_str(&new_chunk);
                result
           }
           else {
            acc
           }
        });
        Ok(to_value(&replaced))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::value::{to_value};

    use errors::TeraError::*;

    use super::*;

    #[test]
    fn test_upper() {
        let result = upper(to_value("hello"), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("HELLO"));
    }

    #[test]
    fn test_upper_error() {
        let result = upper(to_value(&50), HashMap::new());
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            FilterIncorrectArgType("upper".to_string(), "value".to_string(), to_value(&50), "String".to_string())
        );
    }

    #[test]
    fn test_trim() {
        let result = trim(to_value("  hello  "), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("hello"));
    }

    #[test]
    fn test_truncate_smaller_than_length() {
        let mut args = HashMap::new();
        args.insert("length".to_string(), to_value(&255));
        let result = truncate(to_value("hello"), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("hello"));
    }

    #[test]
    fn test_truncate_when_required() {
        let mut args = HashMap::new();
        args.insert("length".to_string(), to_value(&2));
        let result = truncate(to_value("日本語"), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("日本…"));
    }

    #[test]
    fn test_lower() {
        let result = lower(to_value("HELLO"), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("hello"));
    }

    #[test]
    fn test_wordcount() {
        let result = wordcount(to_value("Joel is a slug"), HashMap::new());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value(&4));
    }

    #[test]
    fn test_replace() {
        let mut args = HashMap::new();
        args.insert("old".to_string(), to_value(&"Hello"));
        args.insert("new".to_string(), to_value(&"Goodbye"));
        let result = replace(to_value(&"Hello world!"), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("Goodbye world!"));
    }

    #[test]
    fn test_replace_with_count() {
        let mut args = HashMap::new();
        args.insert("old".to_string(), to_value(&"a"));
        args.insert("new".to_string(), to_value(&"d'oh, "));
        args.insert("count".to_string(), to_value(&2));
        let result = replace(to_value(&"aaaaargh"), args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), to_value("d'oh, d'oh, aaargh"));
    }
}
