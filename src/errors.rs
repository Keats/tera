use std::collections::{LinkedList};

use serde_json::value::Value;

/// Library generic result type.
pub type TeraResult<T> = Result<T, TeraError>;

quick_error! {
    #[derive(PartialEq, Debug, Clone)]
    pub enum TeraError {
        MismatchingEndTag(line_no: usize, col_no: usize, expected: String, found: String) {
            display("Was expecting block with name `{}` to be closed, but `{}` is closing at line {:?}, column {:?}",
                    expected, found, line_no, col_no)
            description("unexpected end tag name")
        }
        InvalidSyntax(line_no: usize, col_no: usize) {
            display("invalid Tera syntax at line {:?}, column {:?}", line_no, col_no)
            description("invalid Tera syntax")
        }
        DeprecatedSyntax(line_no: usize, col_no: usize, message: String) {
            display("deprecated syntax at line {:?}, column {:?}: {}", line_no, col_no, message)
            description("deprecated syntax")
        }

        // Runtime errors
        InvalidValue(name: String) {
            display("Expected the value to be an Object while rendering `{}`.", name)
            description("invalid value")
        }
        TemplateNotFound(name: String) {
            display("Template `{}` was not found", name)
            description("template not found")
        }
        FilterNotFound(name: String) {
            display("Filter `{}` was not found.", name)
            description("filter not found")
        }
        MacroNotFound(name: String, namespace: String) {
            display("Macro `{}` was not found in the namespace `{}`.", name, namespace)
            description("macro not found")
        }
        CircularExtends(template_names: Vec<String>) {
            display("template loop: `{}`", template_names.join("` extends `"))
            description("loop of template extends")
        }
        SuperNoParent(template_name: String, block_name: String) {
            display("super() called in block `{}` of template `{}` that does not extend any other template", block_name, template_name)
            description("no parent template")
        }
        SuperOutsideBlock(template_name: String) {
            display("super() invoked in template `{}` outside any block", template_name)
            description("super invoked without surrounding block")
        }
        NotANumber(name: String) {
            display("Field `{}` was used in a math operation but is not a number", name)
            description("field is not a number")
        }
        NotAnArray(name: String) {
            display("Field `{}` is not an array but was used as iterator in forloop", name)
            description("field is not an array")
        }
        FieldNotFound(name: String) {
            display("Field `{}` was not found in the context.", name)
            description("field not found")
        }
        Internal(message: String) {
            display("Tera encountered an internal error: {}", message)
            description("tera internal error")
        }
        FilterIncorrectArgType(filter_name: String, arg_name: String, arg_value: Value, expected_type: String) {
            display("Filter `{}` received an incorrect type for arg `{}`: got {:?} but expected a {}", filter_name, arg_name, arg_value, expected_type)
            description("incorrect filter arg type")
        }
        FilterMissingArg(filter_name: String, arg_name: String) {
            display("Filter `{}` expected an arg called `{}`", filter_name, arg_name)
            description("missing arg in filter call")
        }
        TesterNotFound(name: String) {
            display("Tester `{}` was not found in the context.", name)
            description("tester not found")
        }
        TestError(tester_name: String, message: String) {
            display("Tester `{}` encountered an error while running: {}", tester_name, message)
            description("tester runtime error")
        }
        MacroCallWrongArgs(macro_name: String, expected_args: LinkedList<String>, args: Vec<String>) {
            display("Macro `{}` got `{:?}` for args but was expecting `{:?}` (order does not matter)", macro_name, expected_args, args)
            description("macro wrong args")
        }
    }
}
