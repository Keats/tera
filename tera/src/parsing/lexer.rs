use std::fmt;

use crate::delimiters::Delimiters;
use crate::errors::Error;
use crate::utils::Span;

// handwritten lexer, peekable iterator/tokenization mostly taken from minijinja

fn memstr(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Will try to go over `-? {name} -?{block_end}`.
/// Returns None if the name doesn't match the tag or the (offset, ws) tuple for the end of the tag
fn skip_tag(block_str: &str, name: &str, block_end: &str) -> Option<(usize, bool)> {
    let mut ptr = block_str;

    if let Some(rest) = ptr.strip_prefix('-') {
        ptr = rest;
    }
    while let Some(rest) = ptr.strip_prefix(|x: char| x.is_ascii_whitespace()) {
        ptr = rest;
    }

    ptr = ptr.strip_prefix(name)?;

    while let Some(rest) = ptr.strip_prefix(|x: char| x.is_ascii_whitespace()) {
        ptr = rest;
    }
    let mut outer_ws = false;
    if let Some(rest) = ptr.strip_prefix('-') {
        ptr = rest;
        outer_ws = true;
    }
    ptr = ptr.strip_prefix(block_end)?;

    Some((block_str.len() - ptr.len(), outer_ws))
}

/// We want to find the next time we see any start marker (variable, block, or comment)
fn find_start_marker(tpl: &str, delimiters: &Delimiters) -> Option<usize> {
    let var_start = delimiters.variable_start.as_bytes();
    let block_start = delimiters.block_start.as_bytes();
    let comment_start = delimiters.comment_start.as_bytes();

    tpl.as_bytes()
        .windows(2)
        .position(|w| w == var_start || w == block_start || w == comment_start)
}

enum State {
    /// Anything not in the other two states
    Template,
    /// In `{{ ... }}` (or the custom delimiters)
    Variable,
    /// In `{% ... %}` (or the custom delimiters)
    Tag,
}

#[derive(PartialEq)]
pub enum Token<'a> {
    Content(&'a str),
    // We handle the raw tag in the lexer but we have to emit a single token for it
    // so this is equivalent to `TagStart(bool), Content(&'a str), TagEnd(bool)`
    // This token will never appear in the parser
    RawContent(bool, &'a str, bool),

    VariableStart(bool),
    VariableEnd(bool),
    TagStart(bool),
    TagEnd(bool),
    // (start, end) of ws - never exposed to the parser
    Comment(bool, bool),
    Ident(&'a str),

    // a string that has been unescaped
    String(String),
    Str(&'a str),
    Integer(i64),
    Float(f64),
    Bool(bool),

    // math
    Mul,
    Div,
    FloorDiv,
    Mod,
    Plus,
    Minus,
    Power,

    // logic
    LessThan,
    // only exists to make parser code a bit easier
    ClosingTagStart, // </
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
    Equal,
    NotEqual,

    // specific to Tera
    Tilde,
    Pipe,
    Assign,

    // Rest
    Dot,
    QuestionMarkDot,
    QuestionMarkLeftBracket,
    Comma,
    Colon,
    Bang,
    LeftBracket,
    RightBracket,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Spread,
}

impl<'a> fmt::Debug for Token<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Content(s) => write!(f, "CONTENT({s:?})"),
            Token::RawContent(ws_start, s, ws_end) => {
                write!(f, "RAW_CONTENT({ws_start}, {s:?}, {ws_end})")
            }
            Token::VariableStart(ws) => write!(f, "VARIABLE_START({ws})"),
            Token::VariableEnd(ws) => write!(f, "VARIABLE_END({ws})"),
            Token::TagStart(ws) => write!(f, "TAG_START({ws})"),
            Token::TagEnd(ws) => write!(f, "TAG_END({ws})"),
            Token::Comment(start, end) => write!(f, "COMMENT({start}, {end})"),
            Token::Ident(i) => write!(f, "IDENT({i})"),
            Token::Str(s) => write!(f, "STRING({s:?})"),
            Token::String(s) => write!(f, "STRING({s:?})"),
            Token::Integer(i) => write!(f, "INTEGER({i:?})"),
            Token::Float(v) => write!(f, "FLOAT({v:?})"),
            Token::Bool(v) => write!(f, "BOOL({v:?})"),
            Token::Plus => write!(f, "PLUS"),
            Token::Minus => write!(f, "MINUS"),
            Token::Mul => write!(f, "MUL"),
            Token::Div => write!(f, "DIV"),
            Token::FloorDiv => write!(f, "FLOORDIV"),
            Token::Power => write!(f, "POWER"),
            Token::Mod => write!(f, "MOD"),
            Token::Bang => write!(f, "BANG"),
            Token::Dot => write!(f, "DOT"),
            Token::QuestionMarkDot => write!(f, "QUESTION_MARK_DOT"),
            Token::QuestionMarkLeftBracket => write!(f, "QUESTION_MARK_LEFT_BRACKET"),
            Token::Comma => write!(f, "COMMA"),
            Token::Colon => write!(f, "COLON"),
            Token::Tilde => write!(f, "TILDE"),
            Token::Assign => write!(f, "ASSIGN"),
            Token::Pipe => write!(f, "PIPE"),
            Token::Equal => write!(f, "EQ"),
            Token::NotEqual => write!(f, "NE"),
            Token::GreaterThan => write!(f, "GT"),
            Token::GreaterThanOrEqual => write!(f, "GTE"),
            Token::LessThan => write!(f, "LT"),
            Token::ClosingTagStart => write!(f, "CLOSING_TAG_START"),
            Token::LessThanOrEqual => write!(f, "LTE"),
            Token::LeftBracket => write!(f, "LEFT_BRACKET"),
            Token::RightBracket => write!(f, "RIGHT_BRACKET"),
            Token::LeftParen => write!(f, "LEFT_PAREN"),
            Token::RightParen => write!(f, "RIGHT_PAREN"),
            Token::LeftBrace => write!(f, "LEFT_BRACE"),
            Token::RightBrace => write!(f, "RIGHT_BRACE"),
            Token::Spread => write!(f, "SPREAD"),
        }
    }
}

/// Used in error messages
impl<'a> fmt::Display for Token<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Content(_) => write!(f, "content",),
            Token::RawContent(_, _, _) => write!(f, "raw content",),
            Token::VariableStart(_) => write!(f, "`{{{{`"),
            Token::VariableEnd(_) => write!(f, "`}}}}`"),
            Token::TagStart(_) => write!(f, "`{{%`"),
            Token::TagEnd(_) => write!(f, "`%}}`"),
            Token::Comment(_, _) => write!(f, "comment`"),
            Token::Ident(_) => write!(f, "identifier"),
            Token::String(_) | Token::Str(_) => write!(f, "string"),
            Token::Integer(_) => write!(f, "integer"),
            Token::Float(_) => write!(f, "float"),
            Token::Bool(_) => write!(f, "bool"),
            Token::Plus => write!(f, "`+`"),
            Token::Minus => write!(f, "`-`"),
            Token::Mul => write!(f, "`*`"),
            Token::Div => write!(f, "`/`"),
            Token::FloorDiv => write!(f, "`//`"),
            Token::Power => write!(f, "`**`"),
            Token::Mod => write!(f, "`%`"),
            Token::Bang => write!(f, "`!`"),
            Token::Dot => write!(f, "`.`"),
            Token::QuestionMarkDot => write!(f, "`?.`"),
            Token::QuestionMarkLeftBracket => write!(f, "`?[`"),
            Token::Comma => write!(f, "`,`"),
            Token::Colon => write!(f, "`:`"),
            Token::Tilde => write!(f, "`~`"),
            Token::Assign => write!(f, "`=`"),
            Token::Pipe => write!(f, "`|`"),
            Token::Equal => write!(f, "`==`"),
            Token::NotEqual => write!(f, "`!="),
            Token::GreaterThan => write!(f, "`>`"),
            Token::GreaterThanOrEqual => write!(f, "`>=`"),
            Token::LessThan => write!(f, "`<`"),
            Token::ClosingTagStart => write!(f, "`</`"),
            Token::LessThanOrEqual => write!(f, "`<=`"),
            Token::LeftBracket => write!(f, "`[`"),
            Token::RightBracket => write!(f, "`]`"),
            Token::LeftParen => write!(f, "`(`"),
            Token::RightParen => write!(f, "`)`"),
            Token::LeftBrace => write!(f, "`{{`"),
            Token::RightBrace => write!(f, "`}}`"),
            Token::Spread => write!(f, "`...`"),
        }
    }
}

fn basic_tokenize(
    input: &str,
    delimiters: Delimiters,
) -> impl Iterator<Item = Result<(Token<'_>, Span), Error>> {
    let mut rest = input;
    let mut stack = vec![State::Template];
    let mut current_line = 1;
    let mut current_col = 0;
    let mut current_byte = 0;
    let mut errored = false;

    macro_rules! syntax_error {
        ($message:expr, $span:expr) => {{
            errored = true;
            return Some(Err(Error::syntax_error($message.to_string(), &$span)));
        }};
    }

    macro_rules! loc {
        () => {
            (current_line, current_col, current_byte)
        };
    }

    macro_rules! make_span {
        ($start:expr) => {{
            let (start_line, start_col, start_byte) = $start;
            Span {
                start_line,
                start_col,
                end_line: current_line,
                end_col: current_col,
                range: start_byte..current_byte,
            }
        }};
    }

    macro_rules! advance {
        ($num_bytes:expr) => {{
            let (skipped, new_rest) = rest.split_at($num_bytes);
            for c in skipped.chars() {
                current_byte += c.len_utf8();
                match c {
                    '\n' => {
                        current_line += 1;
                        current_col = 0;
                    }
                    _ => current_col += 1,
                }
            }
            rest = new_rest;
            skipped
        }};
    }

    macro_rules! check_ws_start {
        () => {{
            if rest.as_bytes().get(2) == Some(&b'-') {
                advance!(3);
                true
            } else {
                advance!(2);
                false
            }
        }};
    }

    macro_rules! lex_number {
        ($is_negative:expr) => {{
            let start_loc = loc!();
            let mut is_float = false;
            let num_len = rest
                .as_bytes()
                .iter()
                .take_while(|&&c| {
                    if !is_float && c == b'.' {
                        is_float = true;
                        true
                    } else {
                        c.is_ascii_digit()
                    }
                })
                .count();
            let num = advance!(num_len);
            if is_float {
                return Some(Ok((
                    Token::Float(match num.parse::<f64>() {
                        Ok(val) => val * if $is_negative { -1.0 } else { 1.0 },
                        Err(_) => syntax_error!("Invalid float", make_span!(start_loc)),
                    }),
                    make_span!(start_loc),
                )));
            } else {
                return Some(Ok((
                    Token::Integer(match num.parse::<i64>() {
                        Ok(val) => val * if $is_negative { -1 } else { 1 },
                        Err(_) => syntax_error!("Invalid Integer", make_span!(start_loc)),
                    }),
                    make_span!(start_loc),
                )));
            }
        }};
    }

    macro_rules! lex_string {
        ($delim:expr) => {{
            let start_loc = loc!();
            let mut has_escapes = false;
            let mut escaped = false;
            let str_len = rest
                .as_bytes()
                .iter()
                .skip(1)
                .take_while(|&&c| {
                    // if we are escaping something, note it and continue
                    if c == b'\\' {
                        has_escapes = true;
                        escaped = true;
                        return true;
                    }
                    // If we were escaping something, continue
                    if escaped == true {
                        escaped = false;
                        return true;
                    }
                    c != $delim
                })
                .count();
            if rest.as_bytes().get(str_len + 1) != Some(&$delim) {
                syntax_error!(
                    &format!(
                        "String opened with `{0}` is missing its closing `{0}`",
                        $delim as char
                    ),
                    make_span!(start_loc)
                )
            }
            let s = advance!(str_len + 2);
            let str_content = &s[1..s.len() - 1];
            if has_escapes {
                // Basic unescaping
                let mut out = String::with_capacity(str_content.len());
                let mut char_iter = str_content.chars();
                while let Some(c) = char_iter.next() {
                    if c == '\\' {
                        match char_iter.next() {
                            None => {
                                syntax_error!("unexpected end of string", make_span!(start_loc))
                            }
                            Some(c2) => match c2 {
                                '"' | '\'' | '/' | '\\' => out.push(c2),
                                'n' => out.push('\n'),
                                't' => out.push('\t'),
                                'r' => out.push('\r'),
                                _ => syntax_error!(
                                    "unexpected escape character",
                                    make_span!(start_loc)
                                ),
                            },
                        }
                    } else {
                        out.push(c);
                    }
                }

                return Some(Ok((Token::String(out), make_span!(start_loc))));
            } else {
                return Some(Ok((Token::Str(str_content), make_span!(start_loc))));
            }
        }};
    }

    std::iter::from_fn(move || {
        loop {
            if rest.is_empty() | errored {
                return None;
            }

            let start_loc = loc!();

            match stack.last() {
                Some(State::Template) => {
                    match rest.get(..2) {
                        Some(s) if s == delimiters.variable_start => {
                            let ws = check_ws_start!();
                            stack.push(State::Variable);
                            return Some(Ok((Token::VariableStart(ws), make_span!(start_loc))));
                        }
                        Some(s) if s == delimiters.block_start => {
                            // If we have a `{% raw %}` block, we ignore everything until we see a `{% endraw %}`
                            // while still respecting whitespace
                            let ws = check_ws_start!();

                            if let Some((mut offset, end_ws_start_tag)) =
                                skip_tag(rest, "raw", &delimiters.block_end)
                            {
                                let body_start_offset = offset;
                                // Then we see whether we find the start of the tag
                                while let Some(block) = memstr(
                                    &rest.as_bytes()[offset..],
                                    delimiters.block_start.as_bytes(),
                                ) {
                                    let body_end_offset = offset + block;
                                    offset += block + 2;
                                    // Check if the tag starts with a {%- so we know we need to end trim the body
                                    let start_ws_end_tag =
                                        rest.as_bytes().get(offset + 1) == Some(&b'-');
                                    if let Some((endraw, ws_end)) =
                                        skip_tag(&rest[offset..], "endraw", &delimiters.block_end)
                                    {
                                        let mut result = &rest[body_start_offset..body_end_offset];
                                        // Then we trim the inner body of the raw tag as needed directly here
                                        if end_ws_start_tag {
                                            result = result.trim_start();
                                        }
                                        if start_ws_end_tag {
                                            result = result.trim_end();
                                        }
                                        advance!(offset + endraw);
                                        return Some(Ok((
                                            Token::RawContent(ws, result, ws_end),
                                            make_span!(start_loc),
                                        )));
                                    }
                                }
                                syntax_error!("unexpected end of raw block", make_span!(start_loc));
                            }

                            stack.push(State::Tag);
                            return Some(Ok((Token::TagStart(ws), make_span!(start_loc))));
                        }
                        Some(s) if s == delimiters.comment_start => {
                            let ws_start = check_ws_start!();
                            if let Some(end_pos) =
                                memstr(rest.as_bytes(), delimiters.comment_end.as_bytes())
                            {
                                let ws_end = if end_pos > 0 {
                                    rest.as_bytes().get(end_pos - 1) == Some(&b'-')
                                } else {
                                    false
                                };
                                advance!(end_pos + 2);
                                return Some(Ok((
                                    Token::Comment(ws_start, ws_end),
                                    make_span!(start_loc),
                                )));
                            } else {
                                syntax_error!(
                                    format!(
                                        "Closing comment tag `{}` not found",
                                        delimiters.comment_end
                                    ),
                                    make_span!(start_loc)
                                );
                            }
                        }
                        _ => {}
                    }

                    let text = match find_start_marker(rest, &delimiters) {
                        Some(start) => advance!(start),
                        None => advance!(rest.len()),
                    };
                    return Some(Ok((Token::Content(text), make_span!(start_loc))));
                }
                Some(State::Variable) | Some(State::Tag) => {
                    // Whitespaces are ignored in there
                    match rest
                        .as_bytes()
                        .iter()
                        .position(|&x| !x.is_ascii_whitespace())
                    {
                        Some(0) => {} // we got something to parse
                        Some(offset) => {
                            advance!(offset); // ignoring some ws
                            continue;
                        }
                        None => {
                            advance!(rest.len());
                            continue;
                        }
                    }

                    // First we check if we are the end of a tag/variable, safe unwrap
                    match stack.last().unwrap() {
                        State::Tag => {
                            // Check for whitespace control: -{block_end}
                            if rest.get(..1) == Some("-")
                                && rest.get(1..3) == Some(delimiters.block_end.as_ref())
                            {
                                stack.pop();
                                advance!(3);
                                return Some(Ok((Token::TagEnd(true), make_span!(start_loc))));
                            }
                            if rest.get(..2) == Some(delimiters.block_end.as_ref()) {
                                stack.pop();
                                advance!(2);
                                return Some(Ok((Token::TagEnd(false), make_span!(start_loc))));
                            }
                        }
                        State::Variable => {
                            // Check for whitespace control: -{variable_end}
                            if rest.get(..1) == Some("-")
                                && rest.get(1..3) == Some(delimiters.variable_end.as_ref())
                            {
                                stack.pop();
                                advance!(3);
                                return Some(Ok((Token::VariableEnd(true), make_span!(start_loc))));
                            }
                            if rest.get(..2) == Some(delimiters.variable_end.as_ref()) {
                                stack.pop();
                                advance!(2);
                                return Some(Ok((
                                    Token::VariableEnd(false),
                                    make_span!(start_loc),
                                )));
                            }
                        }
                        _ => unreachable!(),
                    }

                    // Check for spread operator
                    if let Some(b"...") = rest.as_bytes().get(..3) {
                        advance!(3);
                        return Some(Ok((Token::Spread, make_span!(start_loc))));
                    }

                    // Then the longer operators
                    let op = match rest.as_bytes().get(..2) {
                        Some(b"//") => Some(Token::FloorDiv),
                        Some(b"**") => Some(Token::Power),
                        Some(b"==") => Some(Token::Equal),
                        Some(b"!=") => Some(Token::NotEqual),
                        Some(b">=") => Some(Token::GreaterThanOrEqual),
                        Some(b"<=") => Some(Token::LessThanOrEqual),
                        Some(b"</") => Some(Token::ClosingTagStart),
                        Some(b"?.") => Some(Token::QuestionMarkDot),
                        Some(b"?[") => Some(Token::QuestionMarkLeftBracket),
                        _ => None,
                    };
                    if let Some(op) = op {
                        advance!(2);
                        return Some(Ok((op, make_span!(start_loc))));
                    }

                    // Then the rest of the ops, strings and numbers
                    // strings and numbers will get returned inside the match so only operators are returned
                    let op = match rest.as_bytes().first() {
                        Some(b'+') => Some(Token::Plus),
                        Some(b'-') => Some(Token::Minus),
                        Some(b'*') => Some(Token::Mul),
                        Some(b'/') => Some(Token::Div),
                        Some(b'%') => Some(Token::Mod),
                        Some(b'!') => Some(Token::Bang),
                        Some(b'.') => Some(Token::Dot),
                        Some(b',') => Some(Token::Comma),
                        Some(b':') => Some(Token::Colon),
                        Some(b'~') => Some(Token::Tilde),
                        Some(b'|') => Some(Token::Pipe),
                        Some(b'=') => Some(Token::Assign),
                        Some(b'>') => Some(Token::GreaterThan),
                        Some(b'<') => Some(Token::LessThan),
                        Some(b'(') => Some(Token::LeftParen),
                        Some(b')') => Some(Token::RightParen),
                        Some(b'[') => Some(Token::LeftBracket),
                        Some(b']') => Some(Token::RightBracket),
                        Some(b'{') => Some(Token::LeftBrace),
                        Some(b'}') => Some(Token::RightBrace),
                        Some(b'\'') => lex_string!(b'\''),
                        Some(b'"') => lex_string!(b'"'),
                        Some(b'`') => lex_string!(b'`'),
                        Some(c) if c.is_ascii_digit() => lex_number!(false),
                        _ => None,
                    };
                    if let Some(op) = op {
                        advance!(1);
                        return Some(Ok((op, make_span!(start_loc))));
                    }

                    // Lastly, idents
                    let ident_len = rest
                        .as_bytes()
                        .iter()
                        .enumerate()
                        .take_while(|&(idx, &c)| {
                            if c == b'_' {
                                true
                            } else if idx == 0 {
                                c.is_ascii_alphabetic()
                            } else {
                                c.is_ascii_alphanumeric()
                            }
                        })
                        .count();
                    if ident_len > 0 {
                        let ident = advance!(ident_len);

                        if ident == "true" || ident == "True" {
                            return Some(Ok((Token::Bool(true), make_span!(start_loc))));
                        }
                        if ident == "false" || ident == "False" {
                            return Some(Ok((Token::Bool(false), make_span!(start_loc))));
                        }

                        return Some(Ok((Token::Ident(ident), make_span!(start_loc))));
                    }

                    syntax_error!("Unexpected character", make_span!(start_loc));
                }
                None => unreachable!("Lexer should never be in that state"),
            }
        }
    })
}

/// Automatically removes whitespace around blocks when asked.
fn whitespace_filter<'a, I: Iterator<Item = Result<(Token<'a>, Span), Error>>>(
    iter: I,
) -> impl Iterator<Item = Result<(Token<'a>, Span), Error>> {
    let mut iter = iter.peekable();
    let mut remove_leading_ws = false;

    macro_rules! handle_content_tokens {
        ($data:expr, $span:expr, $remove_leading_ws: expr) => {{
            if remove_leading_ws {
                remove_leading_ws = false;
                $data = $data.trim_start();
            }
            if $remove_leading_ws {
                remove_leading_ws = $remove_leading_ws;
            }

            if matches!(
                iter.peek(),
                Some(Ok((Token::VariableStart(true), _)))
                    | Some(Ok((Token::TagStart(true), _)))
                    | Some(Ok((Token::Comment(true, _), _)))
                    | Some(Ok((Token::RawContent(true, _, _), _)))
            ) {
                $data = $data.trim_end();
            }

            Some(Ok((Token::Content($data), $span)))
        }};
    }

    std::iter::from_fn(move || match iter.next() {
        Some(Ok((Token::Content(mut data), span))) => {
            handle_content_tokens!(data, span, false)
        }
        Some(Ok((Token::RawContent(_, mut data, ws_end), span))) => {
            handle_content_tokens!(data, span, ws_end)
        }
        rv @ Some(Ok((Token::VariableEnd(true), _))) | rv @ Some(Ok((Token::TagEnd(true), _))) => {
            remove_leading_ws = true;
            rv
        }
        Some(Ok((Token::Comment(_, end_ws), span))) => {
            if end_ws {
                remove_leading_ws = true;
            }
            // Empty content nodes will get removed by the parser
            Some(Ok((Token::Content(""), span)))
        }
        other => {
            remove_leading_ws = false;
            other
        }
    })
}

pub fn tokenize(
    input: &str,
    delimiters: Delimiters,
) -> impl Iterator<Item = Result<(Token<'_>, Span), Error>> {
    whitespace_filter(basic_tokenize(input, delimiters))
}
