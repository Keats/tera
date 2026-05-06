use super::utils::normalize_line_endings;
use crate::delimiters::Delimiters;
use crate::parsing::Compiler;
use crate::parsing::parser::Parser;

#[test]
fn compiler_ok() {
    insta::glob!("compiler_inputs/success/*.txt", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        let normalized_contents = normalize_line_endings(&contents);
        let nodes = Parser::new("", &normalized_contents, Delimiters::default())
            .parse()
            .unwrap()
            .nodes;
        let mut compiler = Compiler::new(&path.file_name().unwrap().to_string_lossy());
        compiler.compile(nodes);

        insta::assert_debug_snapshot!(compiler.chunk);
    });
}

#[test]
fn compiler_blocks() {
    insta::glob!("compiler_inputs/blocks/*.txt", |path| {
        let contents = std::fs::read_to_string(path).unwrap();
        let normalized_contents = normalize_line_endings(&contents);
        let nodes = Parser::new("", &normalized_contents, Delimiters::default())
            .parse()
            .unwrap()
            .nodes;
        let mut compiler = Compiler::new(&path.file_name().unwrap().to_string_lossy());
        compiler.compile(nodes);

        let mut s = String::with_capacity(1000);
        s.push_str(&format!("{:?}", compiler.chunk));
        s.push_str("\n\n");

        let mut blocks: Vec<_> = compiler.blocks.into_iter().collect();
        blocks.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, chunk) in blocks {
            s.push_str(&format!(">> Block: {name}\n"));
            s.push_str(&format!("{chunk:?}"));
            s.push_str("---\n");
        }
        insta::assert_snapshot!(s);
    });
}
