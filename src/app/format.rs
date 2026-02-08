/// Represents the text format detected from user input.
/// Stores the delimiter and bracket scheme used for representing vectors and matrices as text.
/// Auto-detected from pasted/typed input, then reused when formatting output text.
#[derive(Debug, Clone, PartialEq)]
pub struct TextFormat {
    /// Delimiter between numbers within a row/vector, e.g. ", " or "," or " " or "\t"
    pub number_delimiter: String,
    /// Opening bracket for each row/vector, e.g. "[" or "(" or "{" or ""
    pub open_vector: String,
    /// Closing bracket for each row/vector, e.g. "]" or ")" or "}" or ""
    pub close_vector: String,
    /// Delimiter between row vectors in a matrix, e.g. ", " or "; " or ","
    pub vector_delimiter: String,
    /// Opening bracket for the whole matrix, e.g. "[" or "{" or ""
    pub open_matrix: String,
    /// Closing bracket for the whole matrix, e.g. "]" or "}" or ""
    pub close_matrix: String,
}

impl Default for TextFormat {
    fn default() -> Self {
        Self {
            number_delimiter: ", ".to_string(),
            open_vector: "[".to_string(),
            close_vector: "]".to_string(),
            vector_delimiter: ", ".to_string(),
            open_matrix: "[".to_string(),
            close_matrix: "]".to_string(),
        }
    }
}

impl TextFormat {
    /// Auto-detect format from raw text input and parse the numeric values.
    ///
    /// Handles common patterns from many languages:
    /// - `[0.0, 0.0, 0.0, 1.0]`       (Rust / Python list / JSON)
    /// - `(0.0, 0.0, 0.0, 1.0)`        (Python tuple)
    /// - `{0.0, 0.0, 0.0, 1.0}`        (Lua / Matlab cell)
    /// - `0.0 0.0 0.0 1.0`             (space-separated)
    /// - `0.0, 0.0, 0.0, 1.0`          (comma-separated, no brackets)
    /// - `np.array([0.0, 0.0, 0.0, 1.0])` (NumPy wrapper)
    /// - `vec![0.0, 0.0, 0.0, 1.0]`    (Rust vec! macro)
    /// - `[0.0; 0.0; 0.0; 1.0]`        (semicolon-separated)
    ///
    /// Returns `(detected_format, parsed_numbers)` on success.
    pub fn detect_and_parse(input: &str) -> Result<(TextFormat, Vec<f64>), String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err("Empty input".to_string());
        }

        // Strip wrapper functions like np.array(...), vec![...], Quaternion(...)
        let content = Self::strip_wrappers(trimmed);

        // Validate bracket balance before parsing
        Self::validate_brackets(content)?;

        // Detect and strip brackets
        let (open_vector, close_vector, inner) = Self::detect_brackets(content);

        // Detect delimiter and parse numbers from inner content
        let (number_delimiter, numbers) = Self::detect_delimiter_and_parse(inner)?;

        if numbers.is_empty() {
            return Err("No numbers found".to_string());
        }

        let format = TextFormat {
            number_delimiter,
            open_vector,
            close_vector,
            vector_delimiter: ", ".to_string(),
            open_matrix: String::new(),
            close_matrix: String::new(),
        };

        Ok((format, numbers))
    }

    /// Auto-detect format from raw text input and parse as a matrix (2D array of numbers).
    ///
    /// Handles common matrix patterns:
    /// - `[[1, 2, 3], [4, 5, 6], [7, 8, 9]]`     (Python list-of-lists / JSON)
    /// - `[1 0 0; 0 1 0; 0 0 1]`                  (Matlab semicolon-separated)
    /// - `{{1, 2, 3}, {4, 5, 6}, {7, 8, 9}}`      (C/Rust nested braces)
    /// - `np.array([[1, 2, 3], [4, 5, 6]])`        (NumPy wrapper)
    /// - `((1, 2, 3), (4, 5, 6))`                  (Python tuple-of-tuples)
    ///
    /// Returns `(detected_format, parsed_rows)` on success.
    pub fn detect_and_parse_matrix(input: &str) -> Result<(TextFormat, Vec<Vec<f64>>), String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err("Empty input".to_string());
        }

        // Strip wrapper functions like np.array(...)
        let content = Self::strip_wrappers(trimmed);

        // Validate bracket balance before parsing
        Self::validate_brackets(content)?;

        // Detect and strip outer (matrix) brackets
        let (open_matrix, close_matrix, inner) = Self::detect_brackets(content);

        let inner = inner.trim();
        if inner.is_empty() {
            return Err("Empty matrix content".to_string());
        }

        // Detect how rows are separated and split into row content strings
        let (open_vector, close_vector, vector_delimiter, row_contents) =
            Self::detect_and_split_rows(inner)?;

        // Parse each row using the same vector-parsing logic
        let mut rows = Vec::new();
        let mut number_delimiter = None;

        for row_str in &row_contents {
            let trimmed = row_str.trim();
            if trimmed.is_empty() {
                continue;
            }
            let (delim, nums) = Self::detect_delimiter_and_parse(trimmed)?;
            if number_delimiter.is_none() {
                number_delimiter = Some(delim);
            }
            rows.push(nums);
        }

        if rows.is_empty() {
            return Err("No rows found in matrix".to_string());
        }

        let format = TextFormat {
            number_delimiter: number_delimiter.unwrap_or_else(|| ", ".to_string()),
            open_vector,
            close_vector,
            vector_delimiter,
            open_matrix,
            close_matrix,
        };

        Ok((format, rows))
    }

    /// Detect how rows are separated in inner matrix content and split into row strings.
    ///
    /// Returns `(open_vector, close_vector, vector_delimiter, row_contents)`.
    ///
    /// Handles two patterns:
    /// - Nested brackets: `[1, 2], [3, 4]` or `{1, 2}, {3, 4}` or `(1, 2), (3, 4)`
    /// - Delimiter-separated (Matlab-style): `1 0 0; 0 1 0; 0 0 1`
    fn detect_and_split_rows(
        inner: &str,
    ) -> Result<(String, String, String, Vec<String>), String> {
        let trimmed = inner.trim();

        // Try nested brackets first
        let bracket_pair = match trimmed.as_bytes().first() {
            Some(b'[') => Some(('[', ']')),
            Some(b'(') => Some(('(', ')')),
            Some(b'{') => Some(('{', '}')),
            _ => None,
        };

        if let Some((open, close)) = bracket_pair {
            let (row_contents, vector_delimiter) =
                Self::split_nested_rows(trimmed, open, close)?;
            if !row_contents.is_empty() {
                return Ok((
                    open.to_string(),
                    close.to_string(),
                    vector_delimiter,
                    row_contents,
                ));
            }
        }

        // Fall back to semicolon-separated rows (Matlab-style)
        if trimmed.contains(';') {
            let vector_delimiter = if trimmed.contains("; ") { "; " } else { ";" };
            let rows: Vec<String> = trimmed
                .split(';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !rows.is_empty() {
                return Ok((
                    String::new(),
                    String::new(),
                    vector_delimiter.to_string(),
                    rows,
                ));
            }
        }

        Err("Could not detect matrix row format (expected nested brackets or semicolon-separated rows)".to_string())
    }

    /// Split inner text into row substrings by finding balanced bracket pairs,
    /// and detect the vector delimiter from the text between closing and opening brackets.
    ///
    /// Input: `[1, 2, 3], [4, 5, 6]`  →  `(vec!["1, 2, 3", "4, 5, 6"], ", ")`
    fn split_nested_rows(
        inner: &str,
        open: char,
        close: char,
    ) -> Result<(Vec<String>, String), String> {
        let mut rows = Vec::new();
        let mut depth = 0;
        let mut content_start = None;
        let mut vector_delimiter: Option<String> = None;
        let mut last_close_end: Option<usize> = None;

        for (i, ch) in inner.char_indices() {
            if ch == open {
                if depth == 0 {
                    // Capture vector_delimiter from text between previous row's close and this open
                    if vector_delimiter.is_none() {
                        if let Some(lce) = last_close_end {
                            vector_delimiter = Some(inner[lce..i].to_string());
                        }
                    }
                    content_start = Some(i + ch.len_utf8());
                }
                depth += 1;
            } else if ch == close {
                depth -= 1;
                if depth == 0 {
                    if let Some(cs) = content_start {
                        rows.push(inner[cs..i].to_string());
                    }
                    content_start = None;
                    last_close_end = Some(i + ch.len_utf8());
                } else if depth < 0 {
                    return Err("Unbalanced brackets in matrix".to_string());
                }
            } else if depth == 0
                && matches!(ch, '(' | ')' | '[' | ']' | '{' | '}')
            {
                return Err(format!(
                    "Inconsistent row bracket types: expected '{}' and '{}', found '{}'",
                    open, close, ch
                ));
            }
        }

        if depth != 0 {
            return Err("Unbalanced brackets in matrix".to_string());
        }

        let delim = vector_delimiter.unwrap_or_else(|| ", ".to_string());
        Ok((rows, delim))
    }

    /// Strip function-call wrappers to reach the core bracket/number content.
    /// e.g. `np.array([1, 2, 3])` → `[1, 2, 3]`
    /// e.g. `vec![1, 2, 3]` → `[1, 2, 3]`
    fn strip_wrappers(text: &str) -> &str {
        let trimmed = text.trim();

        // If already starts with a bracket or a digit/sign, no wrapper to strip
        match trimmed.as_bytes().first() {
            Some(b'[') | Some(b'(') | Some(b'{') => return trimmed,
            Some(b'0'..=b'9') | Some(b'-') | Some(b'+') | Some(b'.') => return trimmed,
            _ => {}
        }

        // Search for the first bracket character and match it to the last matching close
        for (open, close) in &[('[', ']'), ('(', ')'), ('{', '}')] {
            if let (Some(start), Some(end)) = (trimmed.find(*open), trimmed.rfind(*close)) {
                if start < end {
                    return &trimmed[start..=end];
                }
            }
        }

        // No brackets found — return as-is and hope it's bare numbers
        trimmed
    }

    /// Validate that all brackets in the text are properly balanced and nested.
    ///
    /// Uses the classic stack-based "valid parentheses" algorithm.
    /// Returns `Ok(())` if every bracket is matched and properly nested,
    /// or `Err(description)` if the string is malformed.
    fn validate_brackets(text: &str) -> Result<(), String> {
        let mut stack: Vec<char> = Vec::new();

        for ch in text.chars() {
            match ch {
                '(' | '[' | '{' => stack.push(ch),
                ')' | ']' | '}' => {
                    let expected_open = match ch {
                        ')' => '(',
                        ']' => '[',
                        '}' => '{',
                        _ => unreachable!(),
                    };
                    match stack.pop() {
                        None => {
                            return Err(format!(
                                "Unexpected closing bracket '{}'",
                                ch
                            ));
                        }
                        Some(open) if open != expected_open => {
                            return Err(format!(
                                "Mismatched brackets: '{}' closed by '{}'",
                                open, ch
                            ));
                        }
                        _ => {} // correctly matched
                    }
                }
                _ => {}
            }
        }

        if let Some(ch) = stack.last() {
            return Err(format!("Unclosed bracket '{}'", ch));
        }

        Ok(())
    }

    /// Detect outer bracket type and return (open, close, inner_content).
    fn detect_brackets(text: &str) -> (String, String, &str) {
        let trimmed = text.trim();
        if trimmed.len() < 2 {
            return (String::new(), String::new(), trimmed);
        }

        let bytes = trimmed.as_bytes();
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];

        match (first, last) {
            (b'[', b']') => ("[".into(), "]".into(), &trimmed[1..trimmed.len() - 1]),
            (b'(', b')') => ("(".into(), ")".into(), &trimmed[1..trimmed.len() - 1]),
            (b'{', b'}') => ("{".into(), "}".into(), &trimmed[1..trimmed.len() - 1]),
            _ => (String::new(), String::new(), trimmed),
        }
    }

    /// Detect the number delimiter from inner content and parse all numbers.
    fn detect_delimiter_and_parse(inner: &str) -> Result<(String, Vec<f64>), String> {
        let trimmed = inner.trim();
        if trimmed.is_empty() {
            return Err("Empty content".to_string());
        }

        // Priority 1: comma (most explicit array delimiter)
        if trimmed.contains(',') {
            let delimiter = if trimmed.contains(", ") { ", " } else { "," };
            let numbers = Self::split_and_parse(trimmed, ",")?;
            return Ok((delimiter.to_string(), numbers));
        }

        // Priority 2: semicolons (Matlab-style)
        if trimmed.contains(';') {
            let delimiter = if trimmed.contains("; ") { "; " } else { ";" };
            let numbers = Self::split_and_parse(trimmed, ";")?;
            return Ok((delimiter.to_string(), numbers));
        }

        // Priority 3: tabs
        if trimmed.contains('\t') {
            let numbers = Self::split_and_parse(trimmed, "\t")?;
            return Ok(("\t".to_string(), numbers));
        }

        // Priority 4: whitespace (space-separated)
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() {
            return Err("No numbers found".to_string());
        }
        let numbers: Result<Vec<f64>, String> =
            parts.iter().map(|s| Self::parse_single_number(s)).collect();

        Ok((" ".to_string(), numbers?))
    }

    /// Split text by a delimiter, trim each part, and parse as f64.
    fn split_and_parse(text: &str, delimiter: &str) -> Result<Vec<f64>, String> {
        text.split(delimiter)
            .map(|s| Self::parse_single_number(s.trim()))
            .collect()
    }

    /// Parse a single number string, tolerating Rust-style suffixes and digit separators.
    fn parse_single_number(s: &str) -> Result<f64, String> {
        let mut s = s.trim();
        if s.is_empty() {
            return Err("Empty number token".to_string());
        }

        // Strip Rust type suffixes: 1.0f32, 2.5f64
        for suffix in &["f32", "f64"] {
            if let Some(stripped) = s.strip_suffix(suffix) {
                s = stripped;
                break;
            }
        }

        // Remove Rust digit separators (underscores): 1_000.0 → 1000.0
        let cleaned: String = s.chars().filter(|c| *c != '_').collect();

        cleaned
            .parse::<f64>()
            .map_err(|e| format!("Failed to parse '{}': {}", s, e))
    }

    /// Format a slice of f64 values as a vector string using this format's
    /// brackets and delimiters.
    pub fn format_vector(&self, values: &[f64]) -> String {
        let inner: String = values
            .iter()
            .map(|v| format_number(*v))
            .collect::<Vec<_>>()
            .join(&self.number_delimiter);
        format!("{}{}{}", self.open_vector, inner, self.close_vector)
    }

    /// Format a 2D array of f64 values as a matrix string using this format's
    /// brackets and delimiters.
    ///
    /// Each row is formatted as a vector (using `format_vector`), then rows are
    /// joined by `vector_delimiter` and wrapped in matrix brackets. This handles
    /// all styles uniformly:
    /// - Python/JSON:  `[[1, 2, 3], [4, 5, 6]]`
    /// - Matlab:       `[1 0 0; 0 1 0; 0 0 1]`
    /// - C++:          `{{1, 2, 3}, {4, 5, 6}}`
    pub fn format_matrix(&self, rows: &[Vec<f64>]) -> String {
        let row_strs: Vec<String> = rows.iter().map(|row| self.format_vector(row)).collect();
        format!(
            "{}{}{}",
            self.open_matrix,
            row_strs.join(&self.vector_delimiter),
            self.close_matrix
        )
    }
}

// ---------------------------------------------------------------------------
// Rotation-from-string parsing helpers
// ---------------------------------------------------------------------------

use super::rotation::{AxisAngle, Quaternion, Rotation};

/// Quaternion component ordering convention.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuaternionOrder {
    /// x, y, z, w  (default in many robotics frameworks)
    XYZW,
    /// w, x, y, z  (default in some math libraries)
    WXYZ,
}

/// Parse a rotation from a quaternion string like `[0.0, 0.0, 0.0, 1.0]`.
///
/// Accepts any format that [`TextFormat::detect_and_parse`] understands.
/// `order` determines how the four numbers map to quaternion components.
///
/// Returns `(detected_format, rotation)` on success.
pub fn parse_quaternion_str(
    input: &str,
    order: QuaternionOrder,
) -> Result<(TextFormat, Rotation), String> {
    let (fmt, nums) = TextFormat::detect_and_parse(input)?;
    if nums.len() != 4 {
        return Err(format!(
            "Expected 4 numbers for a quaternion, got {}",
            nums.len()
        ));
    }
    let (w, x, y, z) = match order {
        QuaternionOrder::XYZW => (nums[3] as f32, nums[0] as f32, nums[1] as f32, nums[2] as f32),
        QuaternionOrder::WXYZ => (nums[0] as f32, nums[1] as f32, nums[2] as f32, nums[3] as f32),
    };
    let q = Quaternion::try_new(w, x, y, z)?;
    Ok((fmt, Rotation::from(q)))
}

/// Parse a rotation from an axis-angle-3D string like `[0.1, 0.2, 0.3]`.
///
/// The three numbers are interpreted as `axis * angle` (the rotation vector).
/// Accepts any format that [`TextFormat::detect_and_parse`] understands.
///
/// Returns `(detected_format, rotation)` on success.
pub fn parse_axis_angle_3d_str(input: &str) -> Result<(TextFormat, Rotation), String> {
    let (fmt, nums) = TextFormat::detect_and_parse(input)?;
    if nums.len() != 3 {
        return Err(format!(
            "Expected 3 numbers for axis-angle-3D, got {}",
            nums.len()
        ));
    }
    let (ax, ay, az) = (nums[0] as f32, nums[1] as f32, nums[2] as f32);
    let angle = (ax * ax + ay * ay + az * az).sqrt();
    if angle > 1e-10 {
        let aa = AxisAngle::new(ax / angle, ay / angle, az / angle, angle);
        Ok((fmt, Rotation::from(aa)))
    } else {
        Ok((fmt, Rotation::default()))
    }
}

/// Format a single f64 with reasonable precision.
/// Uses up to 6 significant decimal digits, strips trailing zeros,
/// but always keeps at least one decimal place (e.g. "1.0" not "1").
fn format_number(v: f64) -> String {
    let s = format!("{:.6}", v);
    let s = s.trim_end_matches('0');
    if s.ends_with('.') {
        format!("{}0", s)
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===================================================================
    // Helper
    // ===================================================================

    /// Shorthand for asserting format fields on a vector parse.
    fn detect(input: &str) -> (TextFormat, Vec<f64>) {
        TextFormat::detect_and_parse(input).unwrap()
    }

    fn detect_matrix(input: &str) -> (TextFormat, Vec<Vec<f64>>) {
        TextFormat::detect_and_parse_matrix(input).unwrap()
    }

    // ===================================================================
    // 1. TextFormat detection — vector inputs
    // ===================================================================

    #[test]
    fn square_bracket_comma_space() {
        let (fmt, nums) = detect("[1.0, 2.0, 3.0, 4.0]");
        assert_eq!(fmt.open_vector, "[");
        assert_eq!(fmt.close_vector, "]");
        assert_eq!(fmt.number_delimiter, ", ");
        assert_eq!(nums, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn paren_comma_space() {
        let (fmt, nums) = detect("(0.0, 0.0, 0.0, 1.0)");
        assert_eq!(fmt.open_vector, "(");
        assert_eq!(fmt.close_vector, ")");
        assert_eq!(fmt.number_delimiter, ", ");
        assert_eq!(nums, vec![0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn curly_comma_no_space() {
        let (fmt, nums) = detect("{1,2,3,4}");
        assert_eq!(fmt.open_vector, "{");
        assert_eq!(fmt.close_vector, "}");
        assert_eq!(fmt.number_delimiter, ",");
        assert_eq!(nums, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn bare_space_separated() {
        let (fmt, nums) = detect("1.0 2.0 3.0 4.0");
        assert_eq!(fmt.open_vector, "");
        assert_eq!(fmt.close_vector, "");
        assert_eq!(fmt.number_delimiter, " ");
        assert_eq!(nums, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn bare_comma_separated() {
        let (fmt, nums) = detect("1.0, 2.0, 3.0");
        assert_eq!(fmt.open_vector, "");
        assert_eq!(fmt.close_vector, "");
        assert_eq!(fmt.number_delimiter, ", ");
        assert_eq!(nums, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn tab_separated() {
        let (fmt, nums) = detect("1.0\t2.0\t3.0");
        assert_eq!(fmt.number_delimiter, "\t");
        assert_eq!(nums, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn semicolon_separated_vector() {
        let (fmt, nums) = detect("[1.0; 2.0; 3.0; 4.0]");
        assert_eq!(fmt.open_vector, "[");
        assert_eq!(fmt.close_vector, "]");
        assert_eq!(fmt.number_delimiter, "; ");
        assert_eq!(nums, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn numpy_wrapper_stripped() {
        let (fmt, nums) = detect("np.array([0.0, 0.0, 0.0, 1.0])");
        assert_eq!(fmt.open_vector, "[");
        assert_eq!(fmt.close_vector, "]");
        assert_eq!(fmt.number_delimiter, ", ");
        assert_eq!(nums, vec![0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn rust_vec_macro_stripped() {
        let (fmt, nums) = detect("vec![1.0, 2.0, 3.0]");
        assert_eq!(fmt.open_vector, "[");
        assert_eq!(fmt.close_vector, "]");
        assert_eq!(nums, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn rust_f32_suffixes_parsed() {
        let (_, nums) = detect("[1.0f32, 2.5f64, 3.0]");
        assert_eq!(nums, vec![1.0, 2.5, 3.0]);
    }

    #[test]
    fn rust_underscore_separators() {
        let (_, nums) = detect("[1_000.0, 2_000.0]");
        assert_eq!(nums, vec![1000.0, 2000.0]);
    }

    #[test]
    fn negative_numbers() {
        let (_, nums) = detect("[-1.0, 2.0, -3.5]");
        assert_eq!(nums, vec![-1.0, 2.0, -3.5]);
    }

    #[test]
    fn leading_trailing_whitespace() {
        let (fmt, nums) = detect("  [1.0, 2.0, 3.0]  ");
        assert_eq!(fmt.open_vector, "[");
        assert_eq!(nums, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn empty_input_is_err() {
        assert!(TextFormat::detect_and_parse("").is_err());
        assert!(TextFormat::detect_and_parse("   ").is_err());
    }

    #[test]
    fn no_numbers_is_err() {
        assert!(TextFormat::detect_and_parse("[]").is_err());
    }

    #[test]
    fn integers_parsed_as_floats() {
        let (_, nums) = detect("[1, 0, 0, 0]");
        assert_eq!(nums, vec![1.0, 0.0, 0.0, 0.0]);
    }

    // ===================================================================
    // 2. TextFormat detection — matrix inputs
    // ===================================================================

    #[test]
    fn matrix_nested_square_brackets() {
        let (fmt, rows) = detect_matrix("[[1, 2, 3], [4, 5, 6], [7, 8, 9]]");
        assert_eq!(fmt.open_matrix, "[");
        assert_eq!(fmt.close_matrix, "]");
        assert_eq!(fmt.open_vector, "[");
        assert_eq!(fmt.close_vector, "]");
        assert_eq!(fmt.number_delimiter, ", ");
        assert_eq!(fmt.vector_delimiter, ", ");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec![1.0, 2.0, 3.0]);
        assert_eq!(rows[2], vec![7.0, 8.0, 9.0]);
    }

    #[test]
    fn matrix_nested_curly_braces() {
        let (fmt, rows) = detect_matrix("{{1, 2}, {3, 4}}");
        assert_eq!(fmt.open_matrix, "{");
        assert_eq!(fmt.close_matrix, "}");
        assert_eq!(fmt.open_vector, "{");
        assert_eq!(fmt.close_vector, "}");
        assert_eq!(fmt.vector_delimiter, ", ");
        assert_eq!(rows, vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    }

    #[test]
    fn matrix_matlab_semicolons() {
        let (fmt, rows) = detect_matrix("[1 0 0; 0 1 0; 0 0 1]");
        assert_eq!(fmt.open_matrix, "[");
        assert_eq!(fmt.close_matrix, "]");
        // Matlab-style: open_vector is empty, rows separated by "; "
        assert_eq!(fmt.open_vector, "");
        assert_eq!(fmt.close_vector, "");
        assert_eq!(fmt.vector_delimiter, "; ");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec![1.0, 0.0, 0.0]);
    }

    #[test]
    fn matrix_numpy_wrapper() {
        let (fmt, rows) = detect_matrix("np.array([[1, 0], [0, 1]])");
        assert_eq!(fmt.open_matrix, "[");
        assert_eq!(fmt.open_vector, "[");
        assert_eq!(fmt.vector_delimiter, ", ");
        assert_eq!(rows, vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
    }

    #[test]
    fn matrix_tuple_of_tuples() {
        let (fmt, rows) = detect_matrix("((1.0, 2.0), (3.0, 4.0))");
        assert_eq!(fmt.open_matrix, "(");
        assert_eq!(fmt.close_matrix, ")");
        assert_eq!(fmt.open_vector, "(");
        assert_eq!(fmt.close_vector, ")");
        assert_eq!(fmt.vector_delimiter, ", ");
        assert_eq!(rows, vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    }

    #[test]
    fn matrix_empty_is_err() {
        assert!(TextFormat::detect_and_parse_matrix("").is_err());
        assert!(TextFormat::detect_and_parse_matrix("[]").is_err());
    }

    // ===================================================================
    // 3. format_number helper
    // ===================================================================

    #[test]
    fn format_number_trailing_zeros_stripped() {
        assert_eq!(format_number(1.0), "1.0");
        assert_eq!(format_number(1.5), "1.5");
        assert_eq!(format_number(0.123456), "0.123456");
    }

    #[test]
    fn format_number_negative() {
        assert_eq!(format_number(-1.0), "-1.0");
    }

    // ===================================================================
    // 4. Round-trip: detect → format → detect gives same values
    // ===================================================================

    #[test]
    fn round_trip_vector() {
        let input = "(0.1, 0.2, 0.3, 0.4)";
        let (fmt, nums) = detect(input);
        let formatted = fmt.format_vector(&nums);
        let (_, nums2) = detect(&formatted);
        for (a, b) in nums.iter().zip(nums2.iter()) {
            assert!((a - b).abs() < 1e-6, "round-trip mismatch: {} vs {}", a, b);
        }
    }

    #[test]
    fn round_trip_matrix() {
        let input = "[[1, 0, 0], [0, 1, 0], [0, 0, 1]]";
        let (fmt, rows) = detect_matrix(input);
        let formatted = fmt.format_matrix(&rows);
        let (_, rows2) = detect_matrix(&formatted);
        assert_eq!(rows.len(), rows2.len());
        for (r1, r2) in rows.iter().zip(rows2.iter()) {
            for (a, b) in r1.iter().zip(r2.iter()) {
                assert!((a - b).abs() < 1e-6);
            }
        }
    }

    // ===================================================================
    // 5. Rotation parsing helpers
    // ===================================================================

    /// Helper: assert two f32 values are approximately equal.
    fn assert_approx_eq(a: f32, b: f32, tol: f32, msg: &str) {
        assert!(
            (a - b).abs() < tol,
            "{}: {} ≈ {} (diff = {})",
            msg,
            a,
            b,
            (a - b).abs()
        );
    }

    // -----------------------------------------------------------------------
    // Quaternion from string
    // -----------------------------------------------------------------------

    #[test]
    fn quaternion_xyzw_square_brackets() {
        let (_, rot) =
            parse_quaternion_str("[0.0, 0.0, 0.0, 1.0]", QuaternionOrder::XYZW).unwrap();
        let q = rot.as_quaternion();
        assert_approx_eq(q.w, 1.0, 1e-6, "w");
        assert_approx_eq(q.x, 0.0, 1e-6, "x");
        assert_approx_eq(q.y, 0.0, 1e-6, "y");
        assert_approx_eq(q.z, 0.0, 1e-6, "z");
    }

    #[test]
    fn quaternion_wxyz_parens() {
        let (_, rot) =
            parse_quaternion_str("(1.0, 0.0, 0.0, 0.0)", QuaternionOrder::WXYZ).unwrap();
        let q = rot.as_quaternion();
        assert_approx_eq(q.w, 1.0, 1e-6, "w");
        assert_approx_eq(q.x, 0.0, 1e-6, "x");
    }

    #[test]
    fn quaternion_90_deg_about_z_xyzw() {
        // 90° about Z: q = (x=0, y=0, z=sin(45°), w=cos(45°))
        let s = std::f32::consts::FRAC_PI_4.sin();
        let c = std::f32::consts::FRAC_PI_4.cos();
        let input = format!("[0.0, 0.0, {}, {}]", s, c);
        let (_, rot) = parse_quaternion_str(&input, QuaternionOrder::XYZW).unwrap();
        let q = rot.as_quaternion();
        assert_approx_eq(q.w, c, 1e-5, "w");
        assert_approx_eq(q.z, s, 1e-5, "z");
    }

    #[test]
    fn quaternion_wrong_count_is_err() {
        assert!(parse_quaternion_str("[1.0, 2.0, 3.0]", QuaternionOrder::XYZW).is_err());
        assert!(
            parse_quaternion_str("[1.0, 2.0, 3.0, 4.0, 5.0]", QuaternionOrder::XYZW)
                .is_err()
        );
    }

    #[test]
    fn quaternion_zero_is_err() {
        assert!(
            parse_quaternion_str("[0.0, 0.0, 0.0, 0.0]", QuaternionOrder::XYZW).is_err()
        );
    }

    #[test]
    fn quaternion_normalizes_non_unit() {
        // Input is 2x identity — should normalize to identity
        let (_, rot) =
            parse_quaternion_str("[0, 0, 0, 2]", QuaternionOrder::XYZW).unwrap();
        let q = rot.as_quaternion();
        assert_approx_eq(q.w, 1.0, 1e-6, "w");
    }

    // -----------------------------------------------------------------------
    // Axis-angle 3D from string
    // -----------------------------------------------------------------------

    #[test]
    fn axis_angle_3d_zero_rotation() {
        let (_, rot) = parse_axis_angle_3d_str("[0.0, 0.0, 0.0]").unwrap();
        let q = rot.as_quaternion();
        // Should be identity
        assert_approx_eq(q.w, 1.0, 1e-6, "w");
        assert_approx_eq(q.x, 0.0, 1e-6, "x");
        assert_approx_eq(q.y, 0.0, 1e-6, "y");
        assert_approx_eq(q.z, 0.0, 1e-6, "z");
    }

    #[test]
    fn axis_angle_3d_90_deg_about_z() {
        let angle = std::f32::consts::FRAC_PI_2;
        let input = format!("[0.0, 0.0, {}]", angle);
        let (_, rot) = parse_axis_angle_3d_str(&input).unwrap();
        let q = rot.as_quaternion();
        let expected_w = (angle / 2.0).cos();
        let expected_z = (angle / 2.0).sin();
        assert_approx_eq(q.w, expected_w, 1e-5, "w");
        assert_approx_eq(q.x, 0.0, 1e-5, "x");
        assert_approx_eq(q.y, 0.0, 1e-5, "y");
        assert_approx_eq(q.z, expected_z, 1e-5, "z");
    }

    #[test]
    fn axis_angle_3d_wrong_count_is_err() {
        assert!(parse_axis_angle_3d_str("[1.0, 2.0]").is_err());
        assert!(parse_axis_angle_3d_str("[1.0, 2.0, 3.0, 4.0]").is_err());
    }

    #[test]
    fn axis_angle_3d_space_separated() {
        let angle = std::f32::consts::FRAC_PI_2;
        let input = format!("0.0 0.0 {}", angle);
        let (_, rot) = parse_axis_angle_3d_str(&input).unwrap();
        let q = rot.as_quaternion();
        let expected_w = (angle / 2.0).cos();
        assert_approx_eq(q.w, expected_w, 1e-5, "w");
    }

    // ===================================================================
    // 6. Uneven / irregular whitespace — vectors
    // ===================================================================

    #[test]
    fn uneven_spaces_around_commas() {
        // Python-style copy-paste where spaces around commas are inconsistent
        let (_, nums) = detect("[1.0,  2.0,   3.0,4.0]");
        assert_eq!(nums, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn spaces_inside_brackets() {
        // Extra whitespace padding inside brackets
        let (fmt, nums) = detect("[  1.0, 2.0, 3.0  ]");
        assert_eq!(fmt.open_vector, "[");
        assert_eq!(fmt.close_vector, "]");
        assert_eq!(nums, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn mixed_spacing_around_commas() {
        // Some commas have space after, some don't, some have multiple
        let (_, nums) = detect("[1.0 , 2.0,  3.0 ,  4.0]");
        assert_eq!(nums, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn multiple_spaces_bare_numbers() {
        // Python `print()` output or Matlab console with ragged spacing
        let (_, nums) = detect("1.0   2.0  3.0    4.0");
        assert_eq!(nums, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn tabs_and_spaces_mixed() {
        let (_, nums) = detect("1.0\t 2.0 \t3.0");
        assert_eq!(nums, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn newlines_in_vector_input() {
        // Multi-line paste from Python REPL
        let (_, nums) = detect("[1.0,\n 2.0,\n 3.0]");
        assert_eq!(nums, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn parens_uneven_whitespace() {
        // Python tuple with uneven spacing
        let (fmt, nums) = detect("( 0.0,  0.0, 0.0,  1.0 )");
        assert_eq!(fmt.open_vector, "(");
        assert_eq!(fmt.close_vector, ")");
        assert_eq!(nums, vec![0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn curly_braces_uneven_whitespace() {
        let (fmt, nums) = detect("{  1, 2,  3 , 4  }");
        assert_eq!(fmt.open_vector, "{");
        assert_eq!(fmt.close_vector, "}");
        assert_eq!(nums, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn numpy_wrapper_inner_whitespace() {
        // np.array with irregular spaces inside
        let (_, nums) = detect("np.array([ 1.0,  2.0,   3.0 ])");
        assert_eq!(nums, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn numpy_wrapper_outer_whitespace() {
        // Whitespace around the whole np.array expression
        let (_, nums) = detect("  np.array([1.0, 2.0, 3.0])  ");
        assert_eq!(nums, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn rust_vec_macro_inner_whitespace() {
        let (_, nums) = detect("vec![ 1.0,  2.0,   3.0 ]");
        assert_eq!(nums, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn semicolons_uneven_whitespace() {
        let (_, nums) = detect("[1.0 ;  2.0;3.0 ; 4.0]");
        assert_eq!(nums, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn bare_commas_uneven_whitespace() {
        // No brackets, comma-separated with ragged spacing
        let (_, nums) = detect("1.0,  2.0,   3.0,4.0");
        assert_eq!(nums, vec![1.0, 2.0, 3.0, 4.0]);
    }

    // ===================================================================
    // 7. Uneven / irregular whitespace — matrices
    // ===================================================================

    #[test]
    fn matrix_nested_uneven_whitespace() {
        // Python list-of-lists with inconsistent spacing
        let (_, rows) = detect_matrix("[ [1,  2, 3] , [ 4,5, 6 ],[7, 8,  9] ]");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec![1.0, 2.0, 3.0]);
        assert_eq!(rows[1], vec![4.0, 5.0, 6.0]);
        assert_eq!(rows[2], vec![7.0, 8.0, 9.0]);
    }

    #[test]
    fn matrix_matlab_uneven_spaces() {
        // Matlab with ragged spacing between numbers and around semicolons
        let (_, rows) = detect_matrix("[1  0  0 ;  0 1  0;  0  0  1]");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec![1.0, 0.0, 0.0]);
        assert_eq!(rows[1], vec![0.0, 1.0, 0.0]);
        assert_eq!(rows[2], vec![0.0, 0.0, 1.0]);
    }

    #[test]
    fn matrix_numpy_inner_whitespace() {
        let (_, rows) = detect_matrix("np.array([ [ 1,  0],  [0, 1 ] ])");
        assert_eq!(rows, vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
    }

    #[test]
    fn matrix_tuple_of_tuples_uneven_whitespace() {
        let (_, rows) = detect_matrix("( ( 1.0 , 2.0) ,( 3.0,  4.0 ) )");
        assert_eq!(rows, vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    }

    #[test]
    fn matrix_multiline_python_style() {
        // Multi-line paste from a Python script or REPL
        let input = "[\n  [1, 2, 3],\n  [4, 5, 6],\n  [7, 8, 9]\n]";
        let (_, rows) = detect_matrix(input);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec![1.0, 2.0, 3.0]);
        assert_eq!(rows[1], vec![4.0, 5.0, 6.0]);
        assert_eq!(rows[2], vec![7.0, 8.0, 9.0]);
    }

    #[test]
    fn matrix_matlab_multiline() {
        // Matlab matrix pasted across lines
        let input = "[1 0 0;\n 0 1 0;\n 0 0 1]";
        let (_, rows) = detect_matrix(input);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec![1.0, 0.0, 0.0]);
        assert_eq!(rows[1], vec![0.0, 1.0, 0.0]);
        assert_eq!(rows[2], vec![0.0, 0.0, 1.0]);
    }

    // ===================================================================
    // 8. Malformed brackets — vectors
    // ===================================================================

    #[test]
    fn malformed_unclosed_square_bracket() {
        let err = TextFormat::detect_and_parse("[1.0, 2.0, 3.0").unwrap_err();
        assert!(err.contains("Unclosed"), "expected unclosed error, got: {}", err);
    }

    #[test]
    fn malformed_unexpected_close_bracket() {
        let err = TextFormat::detect_and_parse("1.0, 2.0, 3.0]").unwrap_err();
        assert!(err.contains("Unexpected"), "expected unexpected error, got: {}", err);
    }

    #[test]
    fn malformed_mismatched_brackets_square_paren() {
        // Opens with [ but closes with )
        let err = TextFormat::detect_and_parse("[1.0, 2.0, 3.0)").unwrap_err();
        assert!(err.contains("Mismatched"), "expected mismatch error, got: {}", err);
    }

    #[test]
    fn malformed_mismatched_brackets_paren_curly() {
        let err = TextFormat::detect_and_parse("(1.0, 2.0, 3.0}").unwrap_err();
        assert!(err.contains("Mismatched"), "expected mismatch error, got: {}", err);
    }

    #[test]
    fn malformed_interleaved_brackets() {
        // Classic interleaving: ( [ ) ]
        let err = TextFormat::detect_and_parse("(1.0, [2.0), 3.0]").unwrap_err();
        assert!(err.contains("Mismatched"), "expected mismatch error, got: {}", err);
    }

    #[test]
    fn malformed_unclosed_curly() {
        let err = TextFormat::detect_and_parse("{1.0, 2.0").unwrap_err();
        assert!(err.contains("Unclosed"), "expected unclosed error, got: {}", err);
    }

    #[test]
    fn malformed_extra_open_paren() {
        // Two opens, one close
        let err = TextFormat::detect_and_parse("((1.0, 2.0, 3.0)").unwrap_err();
        assert!(err.contains("Unclosed"), "expected unclosed error, got: {}", err);
    }

    #[test]
    fn malformed_extra_close_bracket() {
        let err = TextFormat::detect_and_parse("[1.0, 2.0]]").unwrap_err();
        assert!(err.contains("Unexpected"), "expected unexpected error, got: {}", err);
    }

    #[test]
    fn malformed_nested_unclosed() {
        // Inner bracket never closed
        let err = TextFormat::detect_and_parse("[1.0, [2.0, 3.0]").unwrap_err();
        assert!(err.contains("Unclosed"), "expected unclosed error, got: {}", err);
    }

    #[test]
    fn malformed_numpy_wrapper_bad_inner() {
        // np.array wrapper with mismatched inner bracket
        let err = TextFormat::detect_and_parse("np.array([1.0, 2.0, 3.0)").unwrap_err();
        assert!(err.contains("Mismatched") || err.contains("Unclosed"),
            "expected bracket error, got: {}", err);
    }

    // ===================================================================
    // 9. Malformed brackets — matrices
    // ===================================================================

    #[test]
    fn malformed_matrix_unclosed_outer() {
        let err = TextFormat::detect_and_parse_matrix("[[1, 2], [3, 4]").unwrap_err();
        assert!(err.contains("Unclosed"), "expected unclosed error, got: {}", err);
    }

    #[test]
    fn malformed_matrix_mismatched_inner() {
        // Inner row bracket mismatched: [3, 4} instead of [3, 4]
        let err = TextFormat::detect_and_parse_matrix("[[1, 2], [3, 4}]").unwrap_err();
        assert!(err.contains("Mismatched"), "expected mismatch error, got: {}", err);
    }

    #[test]
    fn malformed_matrix_interleaved() {
        // Interleaved: [ ( ] )
        let err = TextFormat::detect_and_parse_matrix("[(1, 2], (3, 4))").unwrap_err();
        assert!(err.contains("Mismatched"), "expected mismatch error, got: {}", err);
    }

    #[test]
    fn malformed_matrix_extra_open() {
        let err = TextFormat::detect_and_parse_matrix("[[[1, 2], [3, 4]]").unwrap_err();
        assert!(err.contains("Unclosed"), "expected unclosed error, got: {}", err);
    }

    #[test]
    fn malformed_matrix_mixed_row_bracket_types() {
        // First row uses [] but second uses () — the outer [] is fine but inner
        // brackets are inconsistent and should be detected
        let err = TextFormat::detect_and_parse_matrix("[[1, 2], (3, 4)]").unwrap_err();
        assert!(err.contains("Inconsistent"),
            "expected inconsistent bracket error, got: {}", err);
    }

    // ===================================================================
    // 10. Malformed brackets — quaternion / axis-angle parse helpers
    // ===================================================================

    #[test]
    fn malformed_quaternion_unclosed_bracket() {
        let err = parse_quaternion_str("[0.0, 0.0, 0.0, 1.0", QuaternionOrder::XYZW).unwrap_err();
        assert!(err.contains("Unclosed"), "expected unclosed error, got: {}", err);
    }

    #[test]
    fn malformed_quaternion_mismatched_bracket() {
        let err = parse_quaternion_str("[0.0, 0.0, 0.0, 1.0)", QuaternionOrder::XYZW).unwrap_err();
        assert!(err.contains("Mismatched"), "expected mismatch error, got: {}", err);
    }

    #[test]
    fn malformed_quaternion_interleaved() {
        let err = parse_quaternion_str("(0.0, [0.0), 0.0, 1.0]", QuaternionOrder::XYZW).unwrap_err();
        assert!(err.contains("Mismatched"), "expected mismatch error, got: {}", err);
    }

    #[test]
    fn malformed_axis_angle_unclosed_bracket() {
        let err = parse_axis_angle_3d_str("[0.1, 0.2, 0.3").unwrap_err();
        assert!(err.contains("Unclosed"), "expected unclosed error, got: {}", err);
    }

    #[test]
    fn malformed_axis_angle_mismatched_bracket() {
        let err = parse_axis_angle_3d_str("{0.1, 0.2, 0.3)").unwrap_err();
        assert!(err.contains("Mismatched"), "expected mismatch error, got: {}", err);
    }

    // ===================================================================
    // 11. Inconsistent delimiters
    // ===================================================================

    #[test]
    fn inconsistent_delimiters_comma_and_semicolon() {
        // Mixing commas and semicolons in a vector — should fail
        assert!(TextFormat::detect_and_parse("[1, 2; 3, 4]").is_err());
    }

    #[test]
    fn inconsistent_delimiters_comma_and_space() {
        // Mixing comma-separated and space-separated — should fail
        assert!(TextFormat::detect_and_parse("[1, 2 3, 4]").is_err());
    }
}
