/// Represents the text format detected from user input.
/// Stores the delimiter and bracket scheme used for representing vectors and matrices as text.
/// Auto-detected from pasted/typed input, then reused when formatting output text.
#[derive(Debug, Clone, PartialEq)]
pub struct TextFormat {
    /// Delimiter between numbers within a vector, e.g. ", " or "," or " " or "\t"
    pub number_delimiter: String,
    /// Opening bracket for vectors, e.g. "[" or "(" or "{" or ""
    pub open_vector: String,
    /// Closing bracket for vectors, e.g. "]" or ")" or "}" or ""
    pub close_vector: String,
    /// Open brace for matrices, e.g. "{" "[" or ""
    pub open_matrix: String,
    /// Closing brace for matrices, e.g. "}" "]" or ""
    pub close_matrix: String,
}

impl Default for TextFormat {
    fn default() -> Self {
        Self {
            number_delimiter: ", ".to_string(),
            open_vector: "[".to_string(),
            close_vector: "]".to_string(),
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

        // Detect and strip outer (matrix) brackets
        let (open_matrix, close_matrix, inner) = Self::detect_brackets(content);

        let inner = inner.trim();
        if inner.is_empty() {
            return Err("Empty matrix content".to_string());
        }

        // Check for nested bracket pattern (list-of-lists): [[...], [...], ...]
        // or {{...}, {...}, ...} or ((...), (...), ...)
        if let Some(result) =
            Self::try_parse_nested_rows(inner, &open_matrix, &close_matrix)
        {
            return result;
        }

        // Check for Matlab-style semicolon-separated rows: [1 0 0; 0 1 0; 0 0 1]
        if inner.contains(';') {
            return Self::parse_semicolon_matrix(inner, open_matrix, close_matrix);
        }

        Err("Could not detect matrix format (expected nested brackets or semicolon-separated rows)".to_string())
    }

    /// Try to parse nested bracket rows like `[1, 2], [3, 4]` from the inner content.
    fn try_parse_nested_rows(
        inner: &str,
        open_matrix: &str,
        close_matrix: &str,
    ) -> Option<Result<(TextFormat, Vec<Vec<f64>>), String>> {
        // Determine which bracket type the inner rows use
        let (row_open, row_close) = if inner.starts_with('[') {
            ('[', ']')
        } else if inner.starts_with('(') {
            ('(', ')')
        } else if inner.starts_with('{') {
            ('{', '}')
        } else {
            return None; // No nested brackets found — not a list-of-lists pattern
        };

        // Split into row chunks by finding balanced bracket pairs
        let row_strings = match Self::split_nested_rows(inner, row_open, row_close) {
            Ok(rows) => rows,
            Err(e) => return Some(Err(e)),
        };

        if row_strings.is_empty() {
            return Some(Err("No rows found in matrix".to_string()));
        }

        let mut rows = Vec::new();
        let mut number_delimiter = None;

        for row_str in &row_strings {
            match Self::detect_delimiter_and_parse(row_str) {
                Ok((delim, nums)) => {
                    if number_delimiter.is_none() {
                        number_delimiter = Some(delim);
                    }
                    rows.push(nums);
                }
                Err(e) => return Some(Err(format!("Failed to parse matrix row: {}", e))),
            }
        }

        let format = TextFormat {
            number_delimiter: number_delimiter.unwrap_or_else(|| ", ".to_string()),
            open_vector: row_open.to_string(),
            close_vector: row_close.to_string(),
            open_matrix: open_matrix.to_string(),
            close_matrix: close_matrix.to_string(),
        };

        Some(Ok((format, rows)))
    }

    /// Split inner text into row substrings based on balanced bracket pairs.
    /// Input: `[1, 2, 3], [4, 5, 6]`  → `vec!["1, 2, 3", "4, 5, 6"]`
    fn split_nested_rows(inner: &str, open: char, close: char) -> Result<Vec<String>, String> {
        let mut rows = Vec::new();
        let mut depth = 0;
        let mut content_start = None;

        for (i, ch) in inner.char_indices() {
            if ch == open {
                if depth == 0 {
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
                } else if depth < 0 {
                    return Err("Unbalanced brackets in matrix".to_string());
                }
            }
        }

        if depth != 0 {
            return Err("Unbalanced brackets in matrix".to_string());
        }

        Ok(rows)
    }

    /// Parse Matlab-style semicolon-separated matrix: `1 0 0; 0 1 0; 0 0 1`
    fn parse_semicolon_matrix(
        inner: &str,
        open_matrix: String,
        close_matrix: String,
    ) -> Result<(TextFormat, Vec<Vec<f64>>), String> {
        let row_strs: Vec<&str> = inner.split(';').collect();
        let mut rows = Vec::new();
        let mut number_delimiter = None;

        for row_str in &row_strs {
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
            number_delimiter: number_delimiter.unwrap_or_else(|| " ".to_string()),
            open_vector: String::new(),
            close_vector: String::new(),
            open_matrix,
            close_matrix,
        };

        Ok((format, rows))
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
    /// Produces different styles based on format fields:
    /// - List-of-lists (open_vector non-empty): `[[1, 2, 3], [4, 5, 6]]`
    /// - Matlab-style (open_vector empty): `[1 0 0; 0 1 0; 0 0 1]`
    pub fn format_matrix(&self, rows: &[Vec<f64>]) -> String {
        if self.open_vector.is_empty() && self.close_vector.is_empty() {
            // Matlab-style: semicolon-separated rows within matrix brackets
            let row_strs: Vec<String> = rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|v| format_number(*v))
                        .collect::<Vec<_>>()
                        .join(&self.number_delimiter)
                })
                .collect();
            format!(
                "{}{}{}",
                self.open_matrix,
                row_strs.join("; "),
                self.close_matrix
            )
        } else {
            // List-of-lists style: each row wrapped in vector brackets
            let row_strs: Vec<String> = rows
                .iter()
                .map(|row| {
                    let inner = row
                        .iter()
                        .map(|v| format_number(*v))
                        .collect::<Vec<_>>()
                        .join(&self.number_delimiter);
                    format!("{}{}{}", self.open_vector, inner, self.close_vector)
                })
                .collect();
            format!(
                "{}{}{}",
                self.open_matrix,
                row_strs.join(", "),
                self.close_matrix
            )
        }
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
        assert_eq!(rows, vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    }

    #[test]
    fn matrix_matlab_semicolons() {
        let (fmt, rows) = detect_matrix("[1 0 0; 0 1 0; 0 0 1]");
        assert_eq!(fmt.open_matrix, "[");
        assert_eq!(fmt.close_matrix, "]");
        // Matlab-style: open_vector is empty
        assert_eq!(fmt.open_vector, "");
        assert_eq!(fmt.close_vector, "");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec![1.0, 0.0, 0.0]);
    }

    #[test]
    fn matrix_numpy_wrapper() {
        let (fmt, rows) = detect_matrix("np.array([[1, 0], [0, 1]])");
        assert_eq!(fmt.open_matrix, "[");
        assert_eq!(fmt.open_vector, "[");
        assert_eq!(rows, vec![vec![1.0, 0.0], vec![0.0, 1.0]]);
    }

    #[test]
    fn matrix_tuple_of_tuples() {
        let (fmt, rows) = detect_matrix("((1.0, 2.0), (3.0, 4.0))");
        assert_eq!(fmt.open_matrix, "(");
        assert_eq!(fmt.close_matrix, ")");
        assert_eq!(fmt.open_vector, "(");
        assert_eq!(fmt.close_vector, ")");
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
}
