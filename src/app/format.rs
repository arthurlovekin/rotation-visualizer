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
        // Reject multiple vectors: leaf bracket pairs are those containing no
        // nested brackets.  More than one leaf means multiple vectors.
        let leaf_count = pairs
            .iter()
            .filter(|&&(open, close, _)| {
                !pairs.iter().any(|&(o2, c2, _)| o2 > open && c2 < close)
            })
            .count();
        if leaf_count > 1 {
            return Err("Multiple vectors found in input".to_string());
        }

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

/// Parses a matrix of numbers from a variety of text formats.
/// Returns an `R`-row by `C`-column matrix as `[[f64; C]; R]`.
///
/// Supported formats:
/// - Nested brackets: `[[1, 0, 0], [0, 1, 0], [0, 0, 1]]`
/// - With wrappers: `np.array([[1, 0], [0, 1]])`, `torch.tensor([[1, 0], [0, 1]])`
/// - Matlab semicolons: `[1 0 0; 0 1 0; 0 0 1]` or bare `1 0 0; 0 1 0; 0 0 1`
/// - Newline-separated rows: `1 0 0\n0 1 0\n0 0 1`
///
/// Flat (1D) inputs are rejected.
pub fn parse_matrix<const R: usize, const C: usize>(
    input: &str,
) -> Result<[[f64; C]; R], String> {
    if input.trim().is_empty() {
        return Err("Empty input".to_string());
    }

    let pairs = validate_brackets(input)?;

    // Find leaf bracket pairs (those not containing any other bracket pair).
    let leaf_pairs: Vec<(usize, usize, char)> = pairs
        .iter()
        .filter(|&&(open, close, _)| {
            !pairs.iter().any(|&(o2, c2, _)| o2 > open && c2 < close)
        })
        .copied()
        .collect();

    // Nesting exists when some bracket pairs are non-leaf (i.e. they wrap other pairs).
    let has_nesting = leaf_pairs.len() < pairs.len();

    let row_contents: Vec<&str> = if has_nesting {
        // Nested bracket format: leaf pairs are the row vectors.
        // Verify all row vectors use the same bracket type.
        let first_bt = leaf_pairs[0].2;
        for &(_, _, bt) in &leaf_pairs[1..] {
            if bt != first_bt {
                return Err(format!(
                    "Inconsistent row vector brackets: '{}' and '{}'",
                    first_bt, bt
                ));
            }
        }

        leaf_pairs
            .iter()
            .map(|&(open, close, _)| &input[open + 1..close])
            .collect()
    } else {
        // No nesting: strip a single outer bracket if present, then split rows
        // by semicolons or newlines.
        let content = if pairs.len() == 1 {
            &input[pairs[0].0 + 1..pairs[0].1]
        } else if pairs.is_empty() {
            input
        } else {
            return Err(
                "Ambiguous matrix format: multiple non-nested bracket groups".to_string(),
            );
        };

        if content.contains(';') {
            content
                .split(';')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect()
        } else if content.contains('\n') {
            content
                .split('\n')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            return Err(
                "No 2D structure detected: expected row vectors, semicolons, or newlines"
                    .to_string(),
            );
        }
    };

    if row_contents.len() != R {
        return Err(format!("Expected {} rows, got {}", R, row_contents.len()));
    }

    let mut result = [[0.0f64; C]; R];
    for (i, row_str) in row_contents.iter().enumerate() {
        let (_, numbers) = detect_delimiter_and_parse(row_str)?;
        if numbers.len() != C {
            return Err(format!(
                "Expected {} columns, got {} in row {}",
                C,
                numbers.len(),
                i
            ));
        }
        result[i].copy_from_slice(&numbers);
    }

    Ok(result)
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

    #[test]
    fn two_bracketed_vectors_is_err() {
        let err = parse_vector_and_format::<3>("[1.0, 2.0, 3.0] [4.0,5.0,6.0]").unwrap_err();
        assert!(err.contains("Multiple"), "expected multiple-vector error, got: {}", err);
    }

    #[test]
    fn two_bracketed_vectors_comma_separated_is_err() {
        let err = parse_vector_and_format::<3>("[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]").unwrap_err();
        assert!(err.contains("Multiple"), "expected multiple-vector error, got: {}", err);
    }

    #[test]
    fn three_paren_vectors_is_err() {
        let err = parse_vector_and_format::<3>("(1, 2, 3) (4, 5, 6) (7.0, 8.0, 9.0)").unwrap_err();
        assert!(err.contains("Multiple"), "expected multiple-vector error, got: {}", err);
    }

    // ===================================================================
    // Matrix parsing tests
    // ===================================================================

    const IDENTITY_3X3: [[f64; 3]; 3] = [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
    ];

    // --- Nested bracket formats ---

    #[test]
    fn matrix_nested_square_brackets_comma() {
        let m = parse_matrix::<3, 3>("[[1, 0, 0], [0, 1, 0], [0, 0, 1]]").unwrap();
        assert_eq!(m, IDENTITY_3X3);
    }

    #[test]
    fn matrix_nested_parens_comma() {
        let m = parse_matrix::<3, 3>("((1, 2, 3), (4, 5, 6), (7, 8, 9))").unwrap();
        assert_eq!(m, [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]);
    }

    #[test]
    fn matrix_nested_curly_comma() {
        let m = parse_matrix::<3, 3>("{{1, 0, 0}, {0, 1, 0}, {0, 0, 1}}").unwrap();
        assert_eq!(m, IDENTITY_3X3);
    }

    #[test]
    fn matrix_nested_space_separated() {
        let m = parse_matrix::<3, 3>("[[1 0 0] [0 1 0] [0 0 1]]").unwrap();
        assert_eq!(m, IDENTITY_3X3);
    }

    #[test]
    fn matrix_nested_multiline() {
        let input = "[[1, 0, 0],\n [0, 1, 0],\n [0, 0, 1]]";
        let m = parse_matrix::<3, 3>(input).unwrap();
        assert_eq!(m, IDENTITY_3X3);
    }

    // --- Wrapper prefixes ---

    #[test]
    fn matrix_numpy_wrapper() {
        let m = parse_matrix::<3, 3>("np.array([[1, 0, 0], [0, 1, 0], [0, 0, 1]])").unwrap();
        assert_eq!(m, IDENTITY_3X3);
    }

    #[test]
    fn matrix_numpy_multiline() {
        let input = "np.array([[ 1.,  0.,  0.],\n         [ 0.,  1.,  0.],\n         [ 0.,  0.,  1.]])";
        let m = parse_matrix::<3, 3>(input).unwrap();
        assert_eq!(m, IDENTITY_3X3);
    }

    #[test]
    fn matrix_torch_tensor_wrapper() {
        let m = parse_matrix::<2, 2>("torch.tensor([[1, 0], [0, 1]])").unwrap();
        assert_eq!(m, [[1.0, 0.0], [0.0, 1.0]]);
    }

    // --- Matlab-style semicolons ---

    #[test]
    fn matrix_matlab_semicolons_spaces() {
        let m = parse_matrix::<3, 3>("[1 0 0; 0 1 0; 0 0 1]").unwrap();
        assert_eq!(m, IDENTITY_3X3);
    }

    #[test]
    fn matrix_matlab_semicolons_commas() {
        let m = parse_matrix::<3, 3>("[1, 0, 0; 0, 1, 0; 0, 0, 1]").unwrap();
        assert_eq!(m, IDENTITY_3X3);
    }

    #[test]
    fn matrix_bare_semicolons() {
        let m = parse_matrix::<3, 3>("1 0 0; 0 1 0; 0 0 1").unwrap();
        assert_eq!(m, IDENTITY_3X3);
    }

    #[test]
    fn matrix_semicolons_trailing() {
        // Trailing semicolon (Matlab allows this)
        let m = parse_matrix::<3, 3>("[1 0 0; 0 1 0; 0 0 1;]").unwrap();
        assert_eq!(m, IDENTITY_3X3);
    }

    // --- Newline-separated rows ---

    #[test]
    fn matrix_newline_space_separated() {
        let m = parse_matrix::<3, 3>("1 0 0\n0 1 0\n0 0 1").unwrap();
        assert_eq!(m, IDENTITY_3X3);
    }

    #[test]
    fn matrix_newline_comma_separated() {
        let m = parse_matrix::<3, 3>("1, 0, 0\n0, 1, 0\n0, 0, 1").unwrap();
        assert_eq!(m, IDENTITY_3X3);
    }

    #[test]
    fn matrix_newline_tab_separated() {
        let m = parse_matrix::<3, 3>("1\t0\t0\n0\t1\t0\n0\t0\t1").unwrap();
        assert_eq!(m, IDENTITY_3X3);
    }

    #[test]
    fn matrix_newline_with_blank_lines() {
        let m = parse_matrix::<3, 3>("\n1 0 0\n\n0 1 0\n\n0 0 1\n").unwrap();
        assert_eq!(m, IDENTITY_3X3);
    }

    #[test]
    fn matrix_newline_trailing_commas() {
        // Rows end with commas (sloppy copy-paste)
        let m = parse_matrix::<3, 3>("1, 0, 0,\n0, 1, 0,\n0, 0, 1").unwrap();
        assert_eq!(m, IDENTITY_3X3);
    }

    // --- Non-square matrices ---

    #[test]
    fn matrix_2x3_nested() {
        let m = parse_matrix::<2, 3>("[[1, 2, 3], [4, 5, 6]]").unwrap();
        assert_eq!(m, [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]);
    }

    #[test]
    fn matrix_3x2_nested() {
        let m = parse_matrix::<3, 2>("[[1, 2], [3, 4], [5, 6]]").unwrap();
        assert_eq!(m, [[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]]);
    }

    // --- Special numbers ---

    #[test]
    fn matrix_negative_numbers() {
        let m = parse_matrix::<2, 2>("[[-1, 0], [0, -1]]").unwrap();
        assert_eq!(m, [[-1.0, 0.0], [0.0, -1.0]]);
    }

    #[test]
    fn matrix_scientific_notation() {
        let m = parse_matrix::<2, 2>("[[1e0, 0], [0, 1e0]]").unwrap();
        assert_eq!(m, [[1.0, 0.0], [0.0, 1.0]]);
    }

    #[test]
    fn matrix_floats() {
        let m = parse_matrix::<2, 2>("[[0.707, -0.707], [0.707, 0.707]]").unwrap();
        assert_eq!(m, [[0.707, -0.707], [0.707, 0.707]]);
    }

    #[test]
    fn matrix_integers_as_floats() {
        let m = parse_matrix::<2, 2>("[[1, 0], [0, 1]]").unwrap();
        assert_eq!(m, [[1.0, 0.0], [0.0, 1.0]]);
    }

    // --- Whitespace variations ---

    #[test]
    fn matrix_nested_uneven_whitespace() {
        let m = parse_matrix::<3, 3>("[[ 1,  0,0],[  0, 1, 0 ],[0,0 ,  1]]").unwrap();
        assert_eq!(m, IDENTITY_3X3);
    }

    #[test]
    fn matrix_leading_trailing_whitespace() {
        let m = parse_matrix::<2, 2>("  [[1, 0], [0, 1]]  ").unwrap();
        assert_eq!(m, [[1.0, 0.0], [0.0, 1.0]]);
    }

    #[test]
    fn matrix_matlab_uneven_whitespace() {
        let m = parse_matrix::<2, 2>("[1  0 ;  0  1]").unwrap();
        assert_eq!(m, [[1.0, 0.0], [0.0, 1.0]]);
    }

    // --- Error cases ---

    #[test]
    fn matrix_empty_input() {
        assert!(parse_matrix::<3, 3>("").is_err());
        assert!(parse_matrix::<3, 3>("   ").is_err());
    }

    #[test]
    fn matrix_wrong_row_count_nested() {
        // 2 row brackets instead of 3
        assert!(parse_matrix::<3, 3>("[[1, 0, 0], [0, 1, 0]]").is_err());
    }

    #[test]
    fn matrix_wrong_column_count_nested() {
        // Rows have 4 elements instead of 3
        let err = parse_matrix::<3, 3>("[[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0]]").unwrap_err();
        assert!(err.contains("columns"), "expected column error, got: {}", err);
    }

    #[test]
    fn matrix_jagged_rows_nested() {
        // Rows have different lengths
        assert!(parse_matrix::<2, 2>("[[1, 2], [3]]").is_err());
    }

    #[test]
    fn matrix_mismatched_brackets() {
        assert!(parse_matrix::<2, 2>("[[1, 0], [0, 1)").is_err());
    }

    #[test]
    fn matrix_no_numbers() {
        assert!(parse_matrix::<2, 2>("[[]]").is_err());
    }

    #[test]
    fn matrix_unclosed_bracket() {
        assert!(parse_matrix::<2, 2>("[[1, 0], [0, 1]").is_err());
    }

    #[test]
    fn matrix_flattened_is_err() {
        assert!(parse_matrix::<3, 3>("1, 2, 3, 4, 5, 6, 7, 8, 9").is_err());
        assert!(parse_matrix::<3, 3>("1 2 3 4 5 6 7 8 9").is_err());
        assert!(parse_matrix::<3, 3>("1\n2\n3\n4\n5\n6\n7\n8\n9").is_err());
        assert!(parse_matrix::<3, 3>("[1, 2, 3, 4, 5, 6, 7, 8, 9]").is_err());
        assert!(parse_matrix::<3, 3>("[1 2 3 4 5 6 7 8 9]").is_err());
        assert!(parse_matrix::<3, 3>("[1\n2\n3\n4\n5\n6\n7\n8\n9]").is_err());
    }
}
