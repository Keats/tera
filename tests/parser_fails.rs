extern crate tera;

use std::io::prelude::*;
use std::fs::File;

use tera::Template;


// Loads a file and parse it
fn assert_fail_parsing(filename: &str, path: &str) {
    let mut f = File::open(path).unwrap();
    let mut input = String::new();
    f.read_to_string(&mut input).unwrap();
    // should panic
    Template::new(filename, &input);
}

#[should_panic]
#[test]
fn test_error_parser_duplicate_block() {
    assert_fail_parsing("duplicate", "tests/parser-failures/duplicate_block.html");
}

#[should_panic]
#[test]
fn test_error_parser_wrong_endblock() {
    assert_fail_parsing("wrong_endblock", "tests/parser-failures/wrong_endblock.html");
}

#[should_panic]
#[test]
fn test_error_parser_missing_endblock_name() {
    assert_fail_parsing("missing_name", "tests/parser-failures/missing_endblock_name.html");
}

#[should_panic]
#[test]
fn test_error_parser_extends_not_at_beginning() {
    assert_fail_parsing("extends", "tests/parser-failures/invalid_extends.html");
}

#[should_panic]
#[test]
fn test_error_parser_invalid_elif() {
    assert_fail_parsing("elif", "tests/parser-failures/invalid_elif.html");
}

#[should_panic]
#[test]
fn test_error_parser_invalid_else() {
    assert_fail_parsing("else", "tests/parser-failures/invalid_else.html");
}

#[should_panic]
#[test]
fn test_error_parser_unterminated_variable_tag() {
    assert_fail_parsing("unterminated", "tests/parser-failures/unterminated.html");
}

#[should_panic]
#[test]
fn test_error_parser_invalid_number() {
    assert_fail_parsing("invalid_number", "tests/parser-failures/invalid_number.html");
}

#[should_panic]
#[test]
fn test_error_parser_invalid_operator() {
    assert_fail_parsing("invalid_operator", "tests/parser-failures/invalid_operator.html");
}

#[should_panic]
#[test]
fn test_error_parser_unexpected_terminator() {
    assert_fail_parsing("unexpected_terminator", "tests/parser-failures/unexpected_terminator.html");
}
