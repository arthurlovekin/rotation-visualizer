/// Represents the text format detected from user input for a vector of numbers.
/// Stores the delimiter and bracket scheme used for representing vectors as text.
/// Auto-detected from pasted/typed input, then reused when formatting output text.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorFormat {
    /// Delimiter char between numbers: ',', ' ', '\t', ';', or '\n'
    pub number_delimiter: char,
    /// Opening bracket char: '[', '(', '{', or ' ' for bare (no brackets)
    pub bracket_type: char,
    /// Prefix before the vector (eg. `np.array(`)
    pub prefix: String,
    /// Suffix after the vector (eg. `)`)
    pub suffix: String,
}

impl Default for VectorFormat {
    fn default() -> Self {
        Self {
            number_delimiter: ',',
            bracket_type: '[',
            prefix: String::new(),
            suffix: String::new(),
        }
    }
}

impl VectorFormat {
    pub fn format_vector(&self, values: &[f64]) -> String {
        let mut result = String::new();
        result.push_str(&self.prefix);
        for (i, value) in values.iter().enumerate() {
            if i > 0 {
                result.push(self.number_delimiter);
            }
        }
        result.push_str(&self.suffix);
        result
    }
}

// Definition of a Vector (in a string)
// An N-Vector is a sequence of exactly N numbers separated by a delimiter and
// enclosed in brackets.
//     - The delimiter could be a space, comma, tab, newline, or semicolon (no mixing of multiple delimiters).
//     - The brackets could be square, parentheses, curly, or none (no mixing of multiple bracket types).
//     - The number could be a floating point number (with a decimal point, eg. `1.23`), an integer (eg. `123`), or a scientific notation number (eg. `1.23e4`).
// Whitespace (including, space, tab, newline) inside the vector is ignored (unless it is the delimiter)
// If there is any type of character before or after the vector (prefix or suffix), it doesn't affect the vector (though it is saved in the VectorFormat)
// Zero or multiple vectors in a string is not allowed.

/// Validates bracket matching using a stack (like the "valid parentheses" problem).
/// Returns a list of matched bracket pairs: (open_pos, close_pos, bracket_type).
fn validate_brackets(input: &str) -> Result<Vec<(usize, usize, char)>, String> {
    let mut stack: Vec<(usize, char)> = Vec::new();
    let mut pairs = Vec::new();

    for (i, ch) in input.char_indices() {
        match ch {
            '[' | '(' | '{' => stack.push((i, ch)),
            ']' | ')' | '}' => {
                let (open_pos, open_ch) = stack.pop().ok_or_else(|| {
                    format!("Unexpected closing bracket '{}' at position {}", ch, i)
                })?;
                let expected = match open_ch {
                    '[' => ']',
                    '(' => ')',
                    '{' => '}',
                    _ => unreachable!(),
                };
                if ch != expected {
                    return Err(format!(
                        "Mismatched brackets: '{}' at position {} closed by '{}' at position {}",
                        open_ch, open_pos, ch, i
                    ));
                }
                pairs.push((open_pos, i, open_ch));
            }
            _ => {}
        }
    }

    if let Some(&(pos, ch)) = stack.last() {
        return Err(format!("Unclosed bracket '{}' at position {}", ch, pos));
    }

    Ok(pairs)
}

/// Trims each part from the iterator, skips empty strings, and parses as f64.
fn parse_number_parts<'a>(parts: impl Iterator<Item = &'a str>) -> Result<Vec<f64>, String> {
    let mut numbers = Vec::new();
    for part in parts {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        let num = trimmed
            .parse::<f64>()
            .map_err(|e| format!("Failed to parse '{}' as a number: {}", trimmed, e))?;
        numbers.push(num);
    }
    if numbers.is_empty() {
        return Err("No numbers found".to_string());
    }
    Ok(numbers)
}

/// Detects the delimiter type from the content string and parses all numbers.
/// Priority order: comma > semicolon > tab > newline > space.
fn detect_delimiter_and_parse(content: &str) -> Result<(char, Vec<f64>), String> {
    let has_comma = content.contains(',');
    let has_semicolon = content.contains(';');
    let has_tab = content.contains('\t');
    let has_newline = content.contains('\n');

    if has_comma {
        let numbers = parse_number_parts(content.split(','))?;
        Ok((',', numbers))
    } else if has_semicolon {
        let numbers = parse_number_parts(content.split(';'))?;
        Ok((';', numbers))
    } else if has_tab {
        let numbers = parse_number_parts(content.split('\t'))?;
        Ok(('\t', numbers))
    } else if has_newline {
        let numbers = parse_number_parts(content.split('\n'))?;
        Ok(('\n', numbers))
    } else {
        // Space-delimited: split_whitespace handles multiple spaces and trimming.
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.is_empty() {
            return Err("No numbers found".to_string());
        }
        let mut numbers = Vec::new();
        for part in &parts {
            let num = part
                .parse::<f64>()
                .map_err(|e| format!("Failed to parse '{}' as a number: {}", part, e))?;
            numbers.push(num);
        }
        Ok((' ', numbers))
    }
}

pub fn parse_vector_and_format<const N: usize>(input: &str) -> Result<([f64; N], VectorFormat), String> {
    if input.trim().is_empty() {
        return Err("Empty input".to_string());
    }

    let pairs = validate_brackets(input)?;

    let (content, bracket_type, prefix, suffix) = if pairs.is_empty() {
        // Bare vector (no brackets): the whole input is the vector content.
        (input, ' ', String::new(), String::new())
    } else {
        // The vector bracket is the innermost matched pair (largest open_pos).
        // Outer brackets (e.g. the parens in `np.array(...)`) become prefix/suffix.
        let &(open_pos, close_pos, bt) = pairs
            .iter()
            .max_by_key(|(open, _, _)| *open)
            .unwrap();
        (
            &input[open_pos + 1..close_pos],
            bt,
            input[..open_pos].to_string(),
            input[close_pos + 1..].to_string(),
        )
    };

    let (delimiter, numbers) = detect_delimiter_and_parse(content)?;

    if numbers.len() != N {
        return Err(format!("Expected {} numbers, got {}", N, numbers.len()));
    }
    let mut arr = [0.0; N];
    arr.copy_from_slice(&numbers);

    Ok((
        arr,
        VectorFormat {
            number_delimiter: delimiter,
            bracket_type,
            prefix,
            suffix,
        },
    ))
}


#[cfg(test)]
mod tests {
    use super::*;

    // ===================================================================
    // 1. VectorFormat detection — vector inputs
    // ===================================================================

    #[test]
    fn square_bracket_comma_space() {
        let input = "[1.0, 2.0, 3.0, 4.0]";
        let (vector, fmt) = parse_vector_and_format::<4>(input).unwrap();
        assert_eq!(fmt.bracket_type, '[');
        assert_eq!(fmt.number_delimiter, ',');
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn paren_comma_space() {
        let input = "(0.0, 0.0, 0.0, 1.0)";
        let (vector, fmt) = parse_vector_and_format::<4>(input).unwrap();
        assert_eq!(fmt.bracket_type, '(');
        assert_eq!(fmt.number_delimiter, ',');
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");
        assert_eq!(vector, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn curly_comma_no_space() {
        let input = "{1,2,3,4}";
        let (vector, fmt) = parse_vector_and_format::<4>(input).unwrap();
        assert_eq!(fmt.bracket_type, '{');
        assert_eq!(fmt.number_delimiter, ',');
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn bare_space_separated() {
        let input = "1.0 2.0 3.0 4.0";
        let (vector, fmt) = parse_vector_and_format::<4>(input).unwrap();
        assert_eq!(fmt.bracket_type, ' ');
        assert_eq!(fmt.number_delimiter, ' ');
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn bare_comma_separated() {
        let input = "1.0, 2.0, 3.0";
        let (vector, fmt) = parse_vector_and_format::<3>(input).unwrap();
        assert_eq!(fmt.bracket_type, ' ');
        assert_eq!(fmt.number_delimiter, ',');
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn tab_separated() {
        let input = "1.0\t2.0\t3.0";
        let (vector, fmt) = parse_vector_and_format::<3>(input).unwrap();
        assert_eq!(fmt.bracket_type, ' ');
        assert_eq!(fmt.number_delimiter, '\t');
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn semicolon_separated_vector() {
        let input = "[1.0; 2.0; 3.0; 4.0]";
        let (vector, fmt) = parse_vector_and_format::<4>(input).unwrap();
        assert_eq!(fmt.bracket_type, '[');
        assert_eq!(fmt.number_delimiter, ';');
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn numpy_wrapper_stripped() {
        let input = "np.array([0.0, 0.0, 0.0, 1.0])";
        let (vector, fmt) = parse_vector_and_format::<4>(input).unwrap();
        assert_eq!(fmt.bracket_type, '[');
        assert_eq!(fmt.number_delimiter, ',');
        assert_eq!(fmt.prefix, "np.array(");
        assert_eq!(fmt.suffix, ")");
        assert_eq!(vector, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn rust_vec_macro_stripped() {
        let input = "vec![1.0, 2.0, 3.0]";
        let (vector, fmt) = parse_vector_and_format::<3>(input).unwrap();
        assert_eq!(fmt.bracket_type, '[');
        assert_eq!(fmt.number_delimiter, ',');
        assert_eq!(fmt.prefix, "vec!");
        assert_eq!(fmt.suffix, "");
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn negative_numbers() {
        let input = "[-1.0, 2.0, -3.5]";
        let (vector, fmt) = parse_vector_and_format::<3>(input).unwrap();
        assert_eq!(fmt.bracket_type, '[');
        assert_eq!(fmt.number_delimiter, ',');
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");
        assert_eq!(vector, [-1.0, 2.0, -3.5]);
    }

    #[test]
    fn leading_trailing_whitespace() {
        let input = "  [1.0, 2.0, 3.0]  ";
        let (vector, fmt) = parse_vector_and_format::<3>(input).unwrap();
        assert_eq!(fmt.bracket_type, '[');
        assert_eq!(fmt.number_delimiter, ',');
        assert_eq!(fmt.prefix, "  ");
        assert_eq!(fmt.suffix, "  ");
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn empty_input_is_err() {
        assert!(parse_vector_and_format::<4>("").is_err());
        assert!(parse_vector_and_format::<4>("   ").is_err());
    }

    #[test]
    fn no_numbers_is_err() {
        assert!(parse_vector_and_format::<4>("[]").is_err());
    }

    #[test]
    fn integers_parsed_as_floats() {
        let input = "[1, 0, 0, 0]";
        let (vector, _) = parse_vector_and_format::<4>(input).unwrap();
        assert_eq!(vector, [1.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn wrong_count_is_err() {
        // Expect 4 but get 3
        assert!(parse_vector_and_format::<4>("[1.0, 2.0, 3.0]").is_err());
        // Expect 3 but get 4
        assert!(parse_vector_and_format::<3>("[1.0, 2.0, 3.0, 4.0]").is_err());
    }

    // ===================================================================
    // 6. Uneven / irregular whitespace — vectors
    // ===================================================================

    #[test]
    fn uneven_spaces_around_commas() {
        // Python-style copy-paste where spaces around commas are inconsistent
        let input = "[1.0,  2.0,   3.0,4.0]";
        let (vector, _) = parse_vector_and_format::<4>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn spaces_inside_brackets() {
        // Extra whitespace padding inside brackets
        let input = "[  1.0, 2.0, 3.0  ]";
        let (vector, fmt) = parse_vector_and_format::<3>(input).unwrap();
        assert_eq!(fmt.bracket_type, '[');
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn mixed_spacing_around_commas() {
        // Some commas have space after, some don't, some have multiple
        let input = "[1.0 , 2.0,  3.0 ,  4.0]";
        let (vector, _) = parse_vector_and_format::<4>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn multiple_spaces_bare_numbers() {
        // Python `print()` output or Matlab console with ragged spacing
        let input = "1.0   2.0  3.0    4.0";
        let (vector, _) = parse_vector_and_format::<4>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn tabs_and_spaces_mixed() {
        let input = "1.0\t 2.0 \t3.0";
        let (vector, fmt) = parse_vector_and_format::<3>(input).unwrap();
        assert_eq!(fmt.bracket_type, ' ');
        assert_eq!(fmt.number_delimiter, '\t');
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn newlines_in_vector_input() {
        // Multi-line paste from Python REPL
        let input = "[1.0,\n 2.0,\n 3.0]";
        let (vector, fmt) = parse_vector_and_format::<3>(input).unwrap();
        assert_eq!(fmt.bracket_type, '[');
        assert_eq!(fmt.number_delimiter, ',');
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn parens_uneven_whitespace() {
        // Python tuple with uneven spacing
        let input = "( 0.0,  0.0, 0.0,  1.0 )";
        let (vector, fmt) = parse_vector_and_format::<4>(input).unwrap();
        assert_eq!(fmt.bracket_type, '(');
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");
        assert_eq!(vector, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn curly_braces_uneven_whitespace() {
        let input = "{  1, 2,  3 , 4  }";
        let (vector, fmt) = parse_vector_and_format::<4>(input).unwrap();
        assert_eq!(fmt.bracket_type, '{');
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn numpy_wrapper_inner_whitespace() {
        // np.array with irregular spaces inside
        let input = "np.array([ 1.0,  2.0,   3.0 ])";
        let (vector, _) = parse_vector_and_format::<3>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn numpy_wrapper_outer_whitespace() {
        // Whitespace around the whole np.array expression
        let input = "  np.array([1.0, 2.0, 3.0])  ";
        let (vector, _) = parse_vector_and_format::<3>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn rust_vec_macro_inner_whitespace() {
        let input = "vec![ 1.0,  2.0,   3.0 ]";
        let (vector, _) = parse_vector_and_format::<3>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn semicolons_uneven_whitespace() {
        let input = "[1.0 ;  2.0;3.0 ; 4.0]";
        let (vector, _) = parse_vector_and_format::<4>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn bare_commas_uneven_whitespace() {
        // No brackets, comma-separated with ragged spacing
        let input = "1.0,  2.0,   3.0,4.0";
        let (vector, _) = parse_vector_and_format::<4>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    // ===================================================================
    // 8. Malformed brackets — vectors
    // ===================================================================

    #[test]
    fn malformed_unclosed_square_bracket() {
        let err = parse_vector_and_format::<3>("[1.0, 2.0, 3.0").unwrap_err();
        assert!(err.contains("Unclosed"), "expected unclosed error, got: {}", err);
    }

    #[test]
    fn malformed_unexpected_close_bracket() {
        let err = parse_vector_and_format::<3>("1.0, 2.0, 3.0]").unwrap_err();
        assert!(err.contains("Unexpected"), "expected unexpected error, got: {}", err);
    }

    #[test]
    fn malformed_mismatched_brackets_square_paren() {
        // Opens with [ but closes with )
        let err = parse_vector_and_format::<3>("[1.0, 2.0, 3.0)").unwrap_err();
        assert!(err.contains("Mismatched"), "expected mismatch error, got: {}", err);
    }

    #[test]
    fn malformed_mismatched_brackets_paren_curly() {
        let err = parse_vector_and_format::<3>("(1.0, 2.0, 3.0}").unwrap_err();
        assert!(err.contains("Mismatched"), "expected mismatch error, got: {}", err);
    }

    #[test]
    fn malformed_interleaved_brackets() {
        // Classic interleaving: ( [ ) ]
        let err = parse_vector_and_format::<3>("(1.0, [2.0), 3.0]").unwrap_err();
        assert!(err.contains("Mismatched"), "expected mismatch error, got: {}", err);
    }

    #[test]
    fn malformed_unclosed_curly() {
        let err = parse_vector_and_format::<2>("{1.0, 2.0").unwrap_err();
        assert!(err.contains("Unclosed"), "expected unclosed error, got: {}", err);
    }

    #[test]
    fn malformed_extra_open_paren() {
        // Two opens, one close
        let err = parse_vector_and_format::<3>("((1.0, 2.0, 3.0)").unwrap_err();
        assert!(err.contains("Unclosed"), "expected unclosed error, got: {}", err);
    }

    #[test]
    fn malformed_extra_close_bracket() {
        let err = parse_vector_and_format::<2>("[1.0, 2.0]]").unwrap_err();
        assert!(err.contains("Unexpected"), "expected unexpected error, got: {}", err);
    }

    #[test]
    fn malformed_nested_unclosed() {
        // Inner bracket never closed
        let err = parse_vector_and_format::<3>("[1.0, [2.0, 3.0]").unwrap_err();
        assert!(err.contains("Unclosed"), "expected unclosed error, got: {}", err);
    }

    #[test]
    fn malformed_numpy_wrapper_bad_inner() {
        // np.array wrapper with mismatched inner bracket
        let err = parse_vector_and_format::<3>("np.array([1.0, 2.0, 3.0)").unwrap_err();
        assert!(err.contains("Mismatched") || err.contains("Unclosed"),
            "expected bracket error, got: {}", err);
    }

    // ===================================================================
    // 11. Inconsistent delimiters (unambiguously malformed vectors)
    // ===================================================================

    #[test]
    fn inconsistent_delimiters_comma_and_semicolon() {
        // Mixing commas and semicolons in a vector — should fail
        assert!(parse_vector_and_format::<4>("[1, 2; 3, 4]").is_err());
    }

    #[test]
    fn inconsistent_delimiters_comma_and_space() {
        // Mixing comma-separated and space-separated — should fail
        assert!(parse_vector_and_format::<4>("[1, 2 3, 4]").is_err());
    }

    #[test]
    fn no_delimeter_is_err() {
        assert!(parse_vector_and_format::<3>("[1.02.03.0]").is_err());
    }
}
