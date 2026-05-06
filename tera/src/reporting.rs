use crate::errors::ReportError;
use crate::utils::Span;

fn get_line_starts(source: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(source.match_indices('\n').map(|(i, _)| i + 1))
        .collect()
}

struct SourceLocation<'a> {
    line: &'a str,
    underline: String,
    start_line: usize,
    start_col: usize,
}

impl<'a> SourceLocation<'a> {
    fn new(source: &'a str, span: &Span) -> Self {
        let line_starts: Vec<_> = get_line_starts(source);
        let start_line = span.start_line;
        let start_col = span.start_col;
        let line = if start_line == line_starts.len() {
            &source[line_starts[start_line - 1]..]
        } else {
            &source[line_starts[start_line - 1]..line_starts[start_line]]
        }
        .trim_end_matches('\n');

        let mut underline = String::with_capacity(100);
        for c in line.chars().take(start_col) {
            match c {
                '\t' => underline.push('\t'),
                _ => underline.push(' '),
            }
        }
        let width = if span.end_col > span.start_col {
            span.end_col - span.start_col
        } else {
            1
        };
        for _ in 0..width {
            underline.push('^');
        }

        Self {
            line,
            underline,
            start_line,
            start_col,
        }
    }
}

pub(crate) fn generate_report(error: &ReportError) -> String {
    let loc = SourceLocation::new(&error.source, &error.span);
    let line_num_width = loc.start_line.to_string().len();
    let padding = " ".repeat(line_num_width);

    let mut output = format!(
        "error: {}\n\
         {padding}--> {}:{}:{}\n\
         {padding} |\n\
         {} | {}\n\
         {padding} | {}",
        error.message,
        error.filename,
        loc.start_line,
        loc.start_col,
        loc.start_line,
        loc.line,
        loc.underline,
    );

    for note in &error.notes {
        let note_loc = SourceLocation::new(&note.source, &note.span);
        let note_line_num_width = note_loc.start_line.to_string().len();
        let note_padding = " ".repeat(note_line_num_width);
        output.push_str(&format!(
            "\n\nnote: {} {}:{}:{}\n\
             {note_padding} |\n\
             {} | {}\n\
             {note_padding} | {}",
            note.label,
            note.filename,
            note_loc.start_line,
            note_loc.start_col,
            note_loc.start_line,
            note_loc.line,
            note_loc.underline,
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_get_line_starts() {
        let source = "foo\nbar\r\n\nbaz";
        let line_starts = get_line_starts(source);
        assert_eq!(
            line_starts,
            [
                0,  // "foo\n"
                4,  // "bar\r\n"
                9,  // ""
                10, // "baz"
            ],
        );
    }
}
