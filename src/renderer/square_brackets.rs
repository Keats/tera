/// Return a Vec of all substrings contained in '[ ]'s
/// Ignore quoted strings and integers.
pub fn pull_out_square_bracket(s: &str) -> Vec<String> {
    let mut chars = s.chars();
    let mut results = vec![];
    loop {
        match chars.next() {
            Some('[') => {
                let c = chars.next().unwrap();
                if c != '"' && c != '\'' {
                    let mut inside_bracket = vec![c];
                    let mut bracket_count = 1;
                    loop {
                        let c = chars.next();
                        match c {
                            Some(']') => bracket_count -= 1,
                            Some('[') => bracket_count += 1,
                            Some(_) => (),
                            None => break,
                        };
                        if bracket_count == 0 {
                            // Only store results which aren't numbers
                            let sub: String = inside_bracket.into_iter().collect();
                            if sub.parse::<usize>().is_err() {
                                results.push(sub);
                            }
                            break;
                        }
                        inside_bracket.push(c.unwrap());
                    }
                }
            }
            None => break,
            _ => (),
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_pull_out_square_bracket() {
        assert_eq!(pull_out_square_bracket("hi"), Vec::<String>::new());
        assert_eq!(pull_out_square_bracket("['hi']"), Vec::<String>::new());
        assert_eq!(pull_out_square_bracket("[hi] a[0]"), vec!["hi"]);
        assert_eq!(pull_out_square_bracket("hi [th[e]['r']e] [fish]"), vec!["th[e]['r']e", "fish"]);
    }
}
