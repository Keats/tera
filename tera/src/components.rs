use std::collections::{BTreeMap, HashMap};
use std::fmt;

use crate::parsing::ast::{ComponentDefinition, Type};
use crate::value::Value;

/// The type of component arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ComponentArgType {
    #[allow(missing_docs)]
    String,
    #[allow(missing_docs)]
    Bool,
    #[allow(missing_docs)]
    Integer,
    #[allow(missing_docs)]
    Float,
    #[allow(missing_docs)]
    Number,
    #[allow(missing_docs)]
    Array,
    #[allow(missing_docs)]
    Map,
    #[allow(missing_docs)]
    Bytes,
}

impl ComponentArgType {
    /// Returns the name of this argument type
    pub fn as_str(&self) -> &'static str {
        match self {
            ComponentArgType::String => "string",
            ComponentArgType::Bool => "bool",
            ComponentArgType::Integer => "integer",
            ComponentArgType::Float => "float",
            ComponentArgType::Number => "number",
            ComponentArgType::Array => "array",
            ComponentArgType::Map => "map",
            ComponentArgType::Bytes => "bytes",
        }
    }
}

impl fmt::Display for ComponentArgType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<Type> for ComponentArgType {
    fn from(t: Type) -> Self {
        match t {
            Type::String => ComponentArgType::String,
            Type::Bool => ComponentArgType::Bool,
            Type::Integer => ComponentArgType::Integer,
            Type::Float => ComponentArgType::Float,
            Type::Number => ComponentArgType::Number,
            Type::Array => ComponentArgType::Array,
            Type::Map => ComponentArgType::Map,
            Type::Bytes => ComponentArgType::Bytes,
        }
    }
}

/// Information about a single component argument.
#[derive(Clone, Debug)]
pub struct ComponentArg {
    name: String,
    default: Option<Value>,
    arg_type: Option<ComponentArgType>,
}

impl ComponentArg {
    /// The argument name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// The default value, if one was specified
    pub fn default(&self) -> Option<&Value> {
        self.default.as_ref()
    }
    /// The type, if one was specified.
    pub fn arg_type(&self) -> Option<ComponentArgType> {
        self.arg_type
    }
    /// Whether this param is required, eg no default value
    pub fn is_required(&self) -> bool {
        self.default.is_none()
    }
}

/// Information about a component definition.
#[derive(Clone, Debug)]
pub struct ComponentInfo {
    name: String,
    args: Vec<ComponentArg>,
    rest_param: Option<String>,
    metadata: BTreeMap<String, Value>,
}

impl ComponentInfo {
    /// The component name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// All declared arguments
    pub fn args(&self) -> HashMap<&str, &ComponentArg> {
        HashMap::from_iter(self.args.iter().map(|arg| (arg.name(), arg)))
    }
    /// The rest parameter name (e.g. `rest` from `...rest`), if any.
    pub fn rest_param(&self) -> Option<&str> {
        self.rest_param.as_deref()
    }
    /// Component metadata
    pub fn metadata(&self) -> &BTreeMap<String, Value> {
        &self.metadata
    }
}

impl From<&ComponentDefinition> for ComponentInfo {
    fn from(def: &ComponentDefinition) -> Self {
        let args = def
            .kwargs
            .iter()
            .map(|(name, arg)| ComponentArg {
                name: name.clone(),
                default: arg.default.clone(),
                arg_type: arg.typ.map(ComponentArgType::from),
            })
            .collect();

        ComponentInfo {
            name: def.name.clone(),
            args,
            rest_param: def.rest_param_name.clone(),
            metadata: def.metadata.clone(),
        }
    }
}
