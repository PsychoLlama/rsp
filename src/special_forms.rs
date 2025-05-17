//! Defines special forms (keywords) for the Lisp interpreter.

/// Array of special form names. These are reserved and cannot be used as variable names in `let`.
pub const SPECIAL_FORMS: &[&str] = &["let", "quote", "fn"];

/// Checks if a given name is a special form.
///
/// # Arguments
/// * `name` - The name to check.
///
/// # Returns
/// `true` if the name is a special form, `false` otherwise.
pub fn is_special_form(name: &str) -> bool {
    SPECIAL_FORMS.contains(&name)
}

// Constants for individual special form names, can be used for matching.
pub const LET: &str = "let";
pub const QUOTE: &str = "quote";
pub const FN: &str = "fn";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_special_form() {
        assert!(is_special_form("let"));
        assert!(is_special_form("quote"));
        assert!(is_special_form("fn"));
        assert!(!is_special_form("if")); // Assuming 'if' is not yet a special form
        assert!(!is_special_form("my-function"));
        assert!(!is_special_form(""));
    }

    #[test]
    fn test_special_form_constants() {
        assert_eq!(LET, "let");
        assert_eq!(QUOTE, "quote");
        assert_eq!(FN, "fn");
    }
}
