/// A 

#[derive(Debug, Clone, PartialEq)]
enum BracketType {
    Square,
    Parentheses,
    Curly,
    None,
}

#[derive(Debug, Clone, PartialEq)]
enum NumberDelimiter {
    Space,
    Comma,
    Tab,
    Semicolon,
    Newline,
}

/// Represents the text format detected from user input for a vector of numbers.
/// Stores the delimiter and bracket scheme used for representing vectors as text.
/// Auto-detected from pasted/typed input, then reused when formatting output text.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorFormat {
    /// Delimiter between numbers, e.g. ", " or "," or " " or "\t"
    pub number_delimiter: NumberDelimiter,
    /// Opening bracket, e.g. "[" or "(" or "{" or ""
    pub bracket_type: BracketType,
    /// Prefix before the vector (eg. `np.array(`)
    pub prefix: String,
    /// Suffix after the vector (eg. `)`)
    pub suffix: String,
}

impl Default for VectorFormat {
    fn default() -> Self {
        Self {
            number_delimiter: NumberDelimiter::Comma,
            bracket_type: BracketType::Square,
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
                result.push_str(&self.number_delimiter.to_string());
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

pub fn parse_vector<const N: usize>(input: &str) -> Result<[f64; N], String> {
    // TODO
}

pub fn parse_vector_format(input: &str) -> Result<VectorFormat, String> {
   // TODO
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
        let fmt = parse_vector_format(input).unwrap();
        assert_eq!(fmt.bracket_type, BracketType::Square);
        assert_eq!(fmt.number_delimiter, NumberDelimiter::Comma);
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");

        let vector = parse_vector::<4>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn paren_comma_space() {
        let input = "(0.0, 0.0, 0.0, 1.0)";
        let fmt = parse_vector_format(input).unwrap();
        assert_eq!(fmt.bracket_type, BracketType::Parentheses);
        assert_eq!(fmt.number_delimiter, NumberDelimiter::Comma);
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");
        
        let vector = parse_vector::<4>(input).unwrap();
        assert_eq!(vector, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn curly_comma_no_space() {
        let input = "{1,2,3,4}";
        let fmt = parse_vector_format(input).unwrap();
        assert_eq!(fmt.bracket_type, BracketType::Curly);
        assert_eq!(fmt.number_delimiter, NumberDelimiter::Comma);
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");

        let vector = parse_vector::<4>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn bare_space_separated() {
        let input = "1.0 2.0 3.0 4.0";
        let fmt = parse_vector_format(input).unwrap();
        assert_eq!(fmt.bracket_type, BracketType::None);
        assert_eq!(fmt.number_delimiter, NumberDelimiter::Space);
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");

        let vector = parse_vector::<4>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn bare_comma_separated() {
        let input = "1.0, 2.0, 3.0";
        let fmt = parse_vector_format(input).unwrap();
        assert_eq!(fmt.bracket_type, BracketType::None);
        assert_eq!(fmt.number_delimiter, NumberDelimiter::Comma);
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");

        let vector = parse_vector::<3>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn tab_separated() {
        let input = "1.0\t2.0\t3.0";
        let fmt = parse_vector_format(input).unwrap();
        assert_eq!(fmt.bracket_type, BracketType::None);
        assert_eq!(fmt.number_delimiter, NumberDelimiter::Tab);
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");

        let vector = parse_vector::<3>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn semicolon_separated_vector() {
        let input = "[1.0; 2.0; 3.0; 4.0]";
        let fmt = parse_vector_format(input).unwrap();
        assert_eq!(fmt.bracket_type, BracketType::Square);
        assert_eq!(fmt.number_delimiter, NumberDelimiter::Semicolon);
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");

        let vector = parse_vector::<4>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn numpy_wrapper_stripped() {
        let input = "np.array([0.0, 0.0, 0.0, 1.0])";
        let fmt = parse_vector_format(input).unwrap();
        assert_eq!(fmt.bracket_type, BracketType::Square);
        assert_eq!(fmt.number_delimiter, NumberDelimiter::Comma);
        assert_eq!(fmt.prefix, "np.array(");
        assert_eq!(fmt.suffix, ")");

        let vector = parse_vector::<4>(input).unwrap();
        assert_eq!(vector, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn rust_vec_macro_stripped() {
        let input = "vec![1.0, 2.0, 3.0]";
        let fmt = parse_vector_format(input).unwrap();
        assert_eq!(fmt.bracket_type, BracketType::Square);
        assert_eq!(fmt.number_delimiter, NumberDelimiter::Comma);
        assert_eq!(fmt.prefix, "vec!");
        assert_eq!(fmt.suffix, "");

        let vector = parse_vector::<3>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn negative_numbers() {
        let input = "[-1.0, 2.0, -3.5]";
        let fmt = parse_vector_format(input).unwrap();
        assert_eq!(fmt.bracket_type, BracketType::Square);
        assert_eq!(fmt.number_delimiter, NumberDelimiter::Comma);
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");

        let vector = parse_vector::<3>(input).unwrap();
        assert_eq!(vector, [-1.0, 2.0, -3.5]);
    }

    #[test]
    fn leading_trailing_whitespace() {
        let input = "  [1.0, 2.0, 3.0]  ";
        let fmt = parse_vector_format(input).unwrap();
        assert_eq!(fmt.bracket_type, BracketType::Square);
        assert_eq!(fmt.number_delimiter, NumberDelimiter::Comma);
        assert_eq!(fmt.prefix, "  ");
        assert_eq!(fmt.suffix, "  ");

        let vector = parse_vector::<3>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn empty_input_is_err() {
        assert!(parse_vector::<4>("").is_err());
        assert!(parse_vector_format("").is_err() || parse_vector::<4>("").is_err());
        assert!(parse_vector_format("   ").is_err() || parse_vector::<4>("   ").is_err());
    }

    #[test]
    fn no_numbers_is_err() {
        assert!(parse_vector::<4>("[]").is_err());
    }

    #[test]
    fn integers_parsed_as_floats() {
        let input = "[1, 0, 0, 0]";
        let vector = parse_vector::<4>(input).unwrap();
        assert_eq!(vector, [1.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn wrong_count_is_err() {
        // Expect 4 but get 3
        assert!(parse_vector::<4>("[1.0, 2.0, 3.0]").is_err());
        // Expect 3 but get 4
        assert!(parse_vector::<3>("[1.0, 2.0, 3.0, 4.0]").is_err());
    }

    // ===================================================================
    // 6. Uneven / irregular whitespace — vectors
    // ===================================================================

    #[test]
    fn uneven_spaces_around_commas() {
        // Python-style copy-paste where spaces around commas are inconsistent
        let input = "[1.0,  2.0,   3.0,4.0]";
        let vector = parse_vector::<4>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn spaces_inside_brackets() {
        // Extra whitespace padding inside brackets
        let input = "[  1.0, 2.0, 3.0  ]";
        let fmt = parse_vector_format(input).unwrap();
        assert_eq!(fmt.bracket_type, BracketType::Square);
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");

        let vector = parse_vector::<3>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn mixed_spacing_around_commas() {
        // Some commas have space after, some don't, some have multiple
        let input = "[1.0 , 2.0,  3.0 ,  4.0]";
        let vector = parse_vector::<4>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn multiple_spaces_bare_numbers() {
        // Python `print()` output or Matlab console with ragged spacing
        let input = "1.0   2.0  3.0    4.0";
        let vector = parse_vector::<4>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn tabs_and_spaces_mixed() {
        let input = "1.0\t 2.0 \t3.0";
        let fmt = parse_vector_format(input).unwrap();
        assert_eq!(fmt.bracket_type, BracketType::None);
        assert_eq!(fmt.number_delimiter, NumberDelimiter::Tab);
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");

        let vector = parse_vector::<3>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn newlines_in_vector_input() {
        // Multi-line paste from Python REPL
        let input = "[1.0,\n 2.0,\n 3.0]";
        let fmt = parse_vector_format(input).unwrap();
        assert_eq!(fmt.bracket_type, BracketType::Square);
        assert_eq!(fmt.number_delimiter, NumberDelimiter::Comma);
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");

        let vector = parse_vector::<3>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn parens_uneven_whitespace() {
        // Python tuple with uneven spacing
        let input = "( 0.0,  0.0, 0.0,  1.0 )";
        let fmt = parse_vector_format(input).unwrap();
        assert_eq!(fmt.bracket_type, BracketType::Parentheses);
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");

        let vector = parse_vector::<4>(input).unwrap();
        assert_eq!(vector, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn curly_braces_uneven_whitespace() {
        let input = "{  1, 2,  3 , 4  }";
        let fmt = parse_vector_format(input).unwrap();
        assert_eq!(fmt.bracket_type, BracketType::Curly);
        assert_eq!(fmt.prefix, "");
        assert_eq!(fmt.suffix, "");

        let vector = parse_vector::<4>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn numpy_wrapper_inner_whitespace() {
        // np.array with irregular spaces inside
        let input = "np.array([ 1.0,  2.0,   3.0 ])";
        let vector = parse_vector::<3>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn numpy_wrapper_outer_whitespace() {
        // Whitespace around the whole np.array expression
        let input = "  np.array([1.0, 2.0, 3.0])  ";
        let vector = parse_vector::<3>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn rust_vec_macro_inner_whitespace() {
        let input = "vec![ 1.0,  2.0,   3.0 ]";
        let vector = parse_vector::<3>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn semicolons_uneven_whitespace() {
        let input = "[1.0 ;  2.0;3.0 ; 4.0]";
        let vector = parse_vector::<4>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn bare_commas_uneven_whitespace() {
        // No brackets, comma-separated with ragged spacing
        let input = "1.0,  2.0,   3.0,4.0";
        let vector = parse_vector::<4>(input).unwrap();
        assert_eq!(vector, [1.0, 2.0, 3.0, 4.0]);
    }

    // ===================================================================
    // 8. Malformed brackets — vectors
    // ===================================================================

    #[test]
    fn malformed_unclosed_square_bracket() {
        let err = parse_vector::<3>("[1.0, 2.0, 3.0").unwrap_err();
        assert!(err.contains("Unclosed"), "expected unclosed error, got: {}", err);
    }

    #[test]
    fn malformed_unexpected_close_bracket() {
        let err = parse_vector::<3>("1.0, 2.0, 3.0]").unwrap_err();
        assert!(err.contains("Unexpected"), "expected unexpected error, got: {}", err);
    }

    #[test]
    fn malformed_mismatched_brackets_square_paren() {
        // Opens with [ but closes with )
        let err = parse_vector::<3>("[1.0, 2.0, 3.0)").unwrap_err();
        assert!(err.contains("Mismatched"), "expected mismatch error, got: {}", err);
    }

    #[test]
    fn malformed_mismatched_brackets_paren_curly() {
        let err = parse_vector::<3>("(1.0, 2.0, 3.0}").unwrap_err();
        assert!(err.contains("Mismatched"), "expected mismatch error, got: {}", err);
    }

    #[test]
    fn malformed_interleaved_brackets() {
        // Classic interleaving: ( [ ) ]
        let err = parse_vector::<3>("(1.0, [2.0), 3.0]").unwrap_err();
        assert!(err.contains("Mismatched"), "expected mismatch error, got: {}", err);
    }

    #[test]
    fn malformed_unclosed_curly() {
        let err = parse_vector::<2>("{1.0, 2.0").unwrap_err();
        assert!(err.contains("Unclosed"), "expected unclosed error, got: {}", err);
    }

    #[test]
    fn malformed_extra_open_paren() {
        // Two opens, one close
        let err = parse_vector::<3>("((1.0, 2.0, 3.0)").unwrap_err();
        assert!(err.contains("Unclosed"), "expected unclosed error, got: {}", err);
    }

    #[test]
    fn malformed_extra_close_bracket() {
        let err = parse_vector::<2>("[1.0, 2.0]]").unwrap_err();
        assert!(err.contains("Unexpected"), "expected unexpected error, got: {}", err);
    }

    #[test]
    fn malformed_nested_unclosed() {
        // Inner bracket never closed
        let err = parse_vector::<3>("[1.0, [2.0, 3.0]").unwrap_err();
        assert!(err.contains("Unclosed"), "expected unclosed error, got: {}", err);
    }

    #[test]
    fn malformed_numpy_wrapper_bad_inner() {
        // np.array wrapper with mismatched inner bracket
        let err = parse_vector::<3>("np.array([1.0, 2.0, 3.0)").unwrap_err();
        assert!(err.contains("Mismatched") || err.contains("Unclosed"),
            "expected bracket error, got: {}", err);
    }

    // ===================================================================
    // 11. Inconsistent delimiters (unambiguously malformed vectors)
    // ===================================================================

    #[test]
    fn inconsistent_delimiters_comma_and_semicolon() {
        // Mixing commas and semicolons in a vector — should fail
        assert!(parse_vector::<4>("[1, 2; 3, 4]").is_err());
    }

    #[test]
    fn inconsistent_delimiters_comma_and_space() {
        // Mixing comma-separated and space-separated — should fail
        assert!(parse_vector::<4>("[1, 2 3, 4]").is_err());
    }

    fn no_delimeter_is_err() {
        assert!(parse_vector::<3>("[1.02.03.0]").is_err());
    }
}
