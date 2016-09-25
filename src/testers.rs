use errors::{TeraResult, TeraError};
use serde_json::value::{Map, Value};
use parser::Node;

use std::collections::LinkedList;

// TODO: Don't expose the AST to tester functions.
pub type TesterFn = fn(context: &Map<String, Value>,
                       value: &Node,
                       params: LinkedList<Value>)
                       -> TeraResult<bool>;

/// Returns true if `value` is defined in the given context. Otherwise, returns
/// false.
pub fn defined(context: &Map<String, Value>, value: &Node, params: LinkedList<Value>)
        -> TeraResult<bool> {
    if params.len() != 0 {
        return Err(TeraError::TestError("defined".to_string(),
            "defined should not be called with parameters".to_string()))
    }

    let name = match *value {
        Node::Identifier { ref name, .. } => name,
        _ => return Err(TeraError::TestError("defined".to_string(),
                "defined can only be called on identifiers".to_string()))
    };

    Ok(context.contains_key(name))
}

