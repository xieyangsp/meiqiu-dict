#[derive(Debug, Clone)]
pub enum SelectionOutcome {
    Text(String),
    NoSelection,
    Unsupported,
}

pub fn is_acceptable_selection(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() || t.len() > 64 {
        return false;
    }
    if t.chars().any(|c| c.is_control()) {
        return false;
    }
    if !t.chars().any(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    t.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '\'' | ' '))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_simple_word() {
        assert!(is_acceptable_selection("hello"));
    }

    #[test]
    fn accepts_hyphenated_and_apostrophe() {
        assert!(is_acceptable_selection("self-aware"));
        assert!(is_acceptable_selection("don't"));
    }

    #[test]
    fn accepts_short_phrase() {
        assert!(is_acceptable_selection("two words"));
    }

    #[test]
    fn trims_whitespace() {
        assert!(is_acceptable_selection("  hello  "));
    }

    #[test]
    fn rejects_empty_and_whitespace() {
        assert!(!is_acceptable_selection(""));
        assert!(!is_acceptable_selection("   "));
    }

    #[test]
    fn rejects_digits_only() {
        assert!(!is_acceptable_selection("12345"));
    }

    #[test]
    fn rejects_control_chars() {
        assert!(!is_acceptable_selection("hello\nworld"));
        assert!(!is_acceptable_selection("hi\tthere"));
    }

    #[test]
    fn rejects_non_ascii() {
        assert!(!is_acceptable_selection("中文"));
        assert!(!is_acceptable_selection("café"));
    }

    #[test]
    fn rejects_too_long() {
        let s: String = "a".repeat(65);
        assert!(!is_acceptable_selection(&s));
    }

    #[test]
    fn rejects_punctuation() {
        assert!(!is_acceptable_selection("hello,"));
        assert!(!is_acceptable_selection("foo."));
    }
}
