mod common;

use common::load_template;


#[should_panic]
#[test]
fn test_error_parser_duplicate_block() {
    load_template("tests/parser-failures/duplicate_block.html");
}

#[should_panic]
#[test]
fn test_error_parser_wrong_endblock() {
    load_template("tests/parser-failures/wrong_endblock.html");
}

#[should_panic]
#[test]
fn test_error_parser_missing_endblock_name() {
    load_template("tests/parser-failures/missing_endblock_name.html");
}

#[should_panic]
#[test]
fn test_error_parser_extends_not_at_beginning() {
    load_template("tests/parser-failures/invalid_extends.html");
}

#[should_panic]
#[test]
fn test_error_parser_invalid_elif() {
    load_template("tests/parser-failures/invalid_elif.html");
}

#[should_panic]
#[test]
fn test_error_parser_invalid_else() {
    load_template("tests/parser-failures/invalid_else.html");
}

#[should_panic]
#[test]
fn test_error_parser_unterminated_variable_tag() {
    load_template("tests/parser-failures/unterminated.html");
}

#[should_panic]
#[test]
fn test_error_parser_invalid_number() {
    load_template("tests/parser-failures/invalid_number.html");
}

#[should_panic]
#[test]
fn test_error_parser_invalid_operator() {
    load_template("tests/parser-failures/invalid_operator.html");
}

#[should_panic]
#[test]
fn test_error_parser_unexpected_terminator() {
    load_template("tests/parser-failures/unexpected_terminator.html");
}

#[should_panic]
#[test]
fn test_error_parser_invalid_content_macro() {
    load_template("tests/parser-failures/invalid_content_macro.html");
}

#[should_panic]
#[test]
fn test_error_parser_missing_not_expression() {
    load_template("tests/parser-failures/missing_not_expression.html");
}
