use lazy_static::lazy_static;
use regex::Regex;
use rustyline::completion::Completer; // Simplified import
use rustyline::highlight::Highlighter; // Removed MatchingBracketHighlighter
use rustyline::hint::Hinter;
// History trait is not directly used by ReplHelper, but by Editor.
// use rustyline::history::History;
use rustyline::Context; // Needed for manual Completer/Hinter impl
use rustyline::error::ReadlineError;
use rustyline::validate::Validator; // Needed for manual Helper impl // Needed for manual Completer/Validator impl

// Removed unused: use rustyline_derive::Helper as RustylineHelperMacro;
use owo_colors::{OwoColorize, Style as OwoStyle}; // For ANSI styling
use rustyline::Helper as RustylineHelperTrait;
use std::borrow::Cow::{self, Owned}; // Helper trait is at the root

lazy_static! {
    // Order matters for matching. More specific regexes should come first if ambiguity exists.
    // Or, process matches based on start position and length.
    // For this highlighter, we'll iterate and apply styles.
    static ref STRING_RE: Regex = Regex::new(r#""([^"\\]|\\.)*""#).unwrap();
    static ref COMMENT_RE: Regex = Regex::new(r";.*").unwrap(); // Lisp comments
    static ref NUMBER_RE: Regex = Regex::new(r"-?\b\d+(\.\d*)?([eE][+-]?\d+)?\b").unwrap();
    // Keywords: special forms and common builtins for distinct highlighting
    static ref KEYWORD_RE: Regex = Regex::new(r"\b(let|if|fn|quote|require|define|lambda|set!|begin|cond|else|=>)\b").unwrap();
    static ref BOOLEAN_NIL_RE: Regex = Regex::new(r"\b(true|false|nil)\b").unwrap();
    // Parentheses and brackets
    static ref PARENS_RE: Regex = Regex::new(r"[(){}\[\]]").unwrap();
    // Symbols: general identifiers, including operators. Needs to be less specific than keywords.
    // This regex is broad; careful ordering or post-processing might be needed if it conflicts.
    static ref SYMBOL_RE: Regex = Regex::new(r"[a-zA-Z_+\-*/<>=!?$%&~^][a-zA-Z0-9_+\-*/<>=!?$%&~^.]*").unwrap();
}

#[derive(Default)]
pub struct LispHighlighter {
    // matching_bracket_highlighter field removed
}

impl Highlighter for LispHighlighter {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        // _pos is for cursor position, not used in this basic ANSI highlighter directly
        // but could be used for cursor-context highlighting.
        let mut highlighted_line = String::with_capacity(line.len() * 2); // Pre-allocate
        let mut current_pos = 0;

        // Define owo-colors styles
        let string_style = OwoStyle::new().green();
        let comment_style = OwoStyle::new().truecolor(128, 128, 128); // DarkGrey
        let number_style = OwoStyle::new().magenta();
        let keyword_style = OwoStyle::new().cyan().bold();
        let boolean_nil_style = OwoStyle::new().yellow();
        let parens_style = OwoStyle::new().blue();
        // Default/Symbol style can be plain or a subtle color
        // let symbol_style = OwoStyle::new().white(); // Example

        // Order matters for matching.
        let tokens_regexes = [
            (&*STRING_RE, Some(string_style)),
            (&*COMMENT_RE, Some(comment_style)),
            (&*NUMBER_RE, Some(number_style)),
            (&*KEYWORD_RE, Some(keyword_style)),
            (&*BOOLEAN_NIL_RE, Some(boolean_nil_style)),
            (&*PARENS_RE, Some(parens_style)),
            // SYMBOL_RE is very broad, handle it as a fallback or make it more specific.
            // For now, unstyled parts will be symbols or plain.
        ];

        // A more robust approach would be to find all non-overlapping matches first,
        // sort them by start position, and then fill in the gaps.
        // This simplified loop processes from left to right.
        while current_pos < line.len() {
            let mut found_match_at_current_pos = false;
            for (regex, style_opt) in &tokens_regexes {
                if let Some(mat) = regex.find_at(line, current_pos) {
                    if mat.start() == current_pos {
                        // Append part before match (should be empty if current_pos is at mat.start())
                        // highlighted_line.push_str(&line[current_pos..mat.start()]);

                        let matched_text = &line[mat.start()..mat.end()];
                        if let Some(style) = style_opt {
                            highlighted_line.push_str(&matched_text.style(*style).to_string());
                        } else {
                            highlighted_line.push_str(matched_text); // No style / default
                        }
                        current_pos = mat.end();
                        found_match_at_current_pos = true;
                        break;
                    }
                }
            }

            if !found_match_at_current_pos {
                // No token matched at current_pos. Advance by one char, append as plain.
                // This part handles symbols or any other text not caught by specific regexes.
                let char_end = if let Some((idx, _)) = line[current_pos..].char_indices().nth(1) {
                    current_pos + idx
                } else {
                    line.len()
                };
                highlighted_line.push_str(&line[current_pos..char_end]);
                current_pos = char_end;
            }
        }
        Owned(highlighted_line)
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: bool) -> bool {
        // Always return true to ensure the main `highlight` method is called on every char change,
        // which will re-apply our owo-colors based syntax highlighting.
        true
    }
}

// We will manually implement Helper and its supertraits.
// The derive macro can be removed if we provide all implementations.
// For now, let's remove it and implement manually.
// #[derive(RustylineHelperMacro)]
pub struct ReplHelper {
    highlighter: LispHighlighter,
    // We could add fields for custom completer, hinter, validator if needed
}

impl ReplHelper {
    pub fn new() -> Self {
        Self {
            highlighter: LispHighlighter::default(),
        }
    }
}

impl Completer for ReplHelper {
    type Candidate = String; // Use String as a simple candidate type

    fn complete(
        &self,
        _line: &str,
        _pos: usize,
        _ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Self::Candidate>), ReadlineError> {
        Ok((0, Vec::new())) // No-op completion
    }
}

impl Hinter for ReplHelper {
    type Hint = String;

    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<Self::Hint> {
        None // No-op hinter
    }
}

impl Highlighter for ReplHelper {
    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        // Return type changed
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize, forced: bool) -> bool {
        self.highlighter.highlight_char(line, pos, forced)
    }
}

impl Validator for ReplHelper {
    fn validate(
        &self,
        ctx: &mut rustyline::validate::ValidationContext,
    ) -> Result<rustyline::validate::ValidationResult, ReadlineError> {
        let input = ctx.input();
        let mut open_parens = 0;
        let mut close_parens = 0;
        let mut in_string = false;
        let mut in_comment = false;

        for ch in input.chars() {
            if in_comment {
                if ch == '\n' {
                    in_comment = false; // Comment ends at newline
                }
                continue; // Ignore characters within a comment for paren counting
            }

            match ch {
                ';' if !in_string => {
                    in_comment = true; // Start of a comment
                }
                '"' => {
                    // Basic string toggle; doesn't handle escaped quotes within strings perfectly
                    // for this simple paren counter, but good enough for now.
                    in_string = !in_string;
                }
                '(' if !in_string => {
                    open_parens += 1;
                }
                ')' if !in_string => {
                    close_parens += 1;
                }
                _ => {}
            }
        }

        if in_string {
            // Unterminated string literal
            Ok(rustyline::validate::ValidationResult::Incomplete)
        } else if open_parens > close_parens {
            // More open parentheses than close parentheses
            Ok(rustyline::validate::ValidationResult::Incomplete)
        } else {
            // Balanced or more closing than opening (which is an error for the parser, but complete for rustyline)
            Ok(rustyline::validate::ValidationResult::Valid(None))
        }
    }
}

// This explicitly states that ReplHelper implements the RustylineHelperTrait marker trait.
// The supertraits (Completer, Hinter, Highlighter, Validator) must be implemented.
impl RustylineHelperTrait for ReplHelper {}

impl Default for ReplHelper {
    fn default() -> Self {
        Self::new()
    }
}
