use serde_json::value::Value;

/// Library generic result type.
pub type TeraResult<T> = Result<T, TeraError>;

quick_error! {
    #[derive(PartialEq, Debug, Clone)]
    pub enum TeraError {
        MismatchingEndBlock(line_no: usize, col_no: usize, expected: String, found: String) {
            display("Was expecting block `{}` to be closed, but `{}` is closing at line {:?}, column {:?}",
                    expected, found, line_no, col_no)
            description("unexpected endblock name")
        }
        InvalidSyntax(line_no: usize, col_no: usize) {
            display("invalid Tera syntax at line {:?}, column {:?}", line_no, col_no)
            description("invalid Tera syntax")
        }
        DeprecatedSyntax(line_no: usize, col_no: usize, message: String) {
            display("deprecated syntax at line {:?}, column {:?}: {}", line_no, col_no, message)
            description("deprecated syntax")
        }
        TemplateNotFound(name: String) {
            display("Template `{}` wasn't found", name)
            description("template not found")
        }
        FilterNotFound(name: String) {
            display("Filter `{}` was not found in the context.", name)
            description("filter not found")
        }

        // Runtime errors
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
        FilterIncorrectArgType(filter_name: String, arg_name: String, arg_value: Value, expected_type: String) {
            display("Filter `{}` received an incorrect type for arg `{}`: got {:?} but expected a {}", filter_name, arg_name, arg_value, expected_type)
            description("incorrect filter arg type")
        }
    }
}
