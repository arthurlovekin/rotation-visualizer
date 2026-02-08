/// Represents the text format detected from user input.
/// Stores the delimiter and bracket scheme used for representing vectors and matrices as text.
/// Auto-detected from pasted/typed input, then reused when formatting output text.
#[derive(Debug, Clone, PartialEq)]
pub struct TextFormat {
    /// Delimiter between numbers within a vector, e.g. ", " or "," or " " or "\t"
    pub number_delimiter: String,
    /// Opening bracket for vectors, e.g. "[" or "(" or "{" or ""
    pub open_bracket: String,
    /// Closing bracket for vectors, e.g. "]" or ")" or "}" or ""
    pub close_bracket: String,
}

impl Default for TextFormat {
    fn default() -> Self {
        Self {
            number_delimiter: ", ".to_string(),
            open_bracket: "[".to_string(),
            close_bracket: "]".to_string(),
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
        let (open_bracket, close_bracket, inner) = Self::detect_brackets(content);

        // Detect delimiter and parse numbers from inner content
        let (number_delimiter, numbers) = Self::detect_delimiter_and_parse(inner)?;

        if numbers.is_empty() {
            return Err("No numbers found".to_string());
        }

        let format = TextFormat {
            number_delimiter,
            open_bracket,
            close_bracket,
        };

        Ok((format, numbers))
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
        format!("{}{}{}", self.open_bracket, inner, self.close_bracket)
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

