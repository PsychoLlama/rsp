use lazy_static::lazy_static;
use regex::Regex;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::completion::{Completer, Candidate as CompletionCandidate}; // Corrected: Candidate
use rustyline::hint::Hinter;
// History trait is not directly used by ReplHelper, but by Editor.
// use rustyline::history::History; 
use rustyline::validate::Validator; // Needed for manual Helper impl
use rustyline::Context; // Needed for manual Completer/Hinter impl
use rustyline::error::ReadlineError; // Needed for manual Completer/Validator impl

// Removed unused: use rustyline_derive::Helper as RustylineHelperMacro;
use std::borrow::Cow::{self, Owned};
use rustyline::style::{Style, Color, Modifier}; // Items are in rustyline::style
use rustyline::styled_text::StyledText;      // Item is in rustyline::styled_text
use rustyline::Helper as RustylineHelperTrait; // Helper trait is at the root

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
    matching_bracket_highlighter: MatchingBracketHighlighter,
}

impl Highlighter for LispHighlighter {
    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, StyledText> {
        let mut styled_text = StyledText::new();
        let mut current_pos = 0;

        // Token definitions with style
        // Order can be important if regexes overlap significantly without careful construction.
        // A more robust tokenizer would produce discrete tokens first.
        // This approach iterates and applies styles based on first match at current_pos.
        let tokens_with_styles = [
            (&*STRING_RE, Style::new().fg_color(Some(Color::Green))),
            (&*COMMENT_RE, Style::new().fg_color(Some(Color::DarkGrey))),
            (&*NUMBER_RE, Style::new().fg_color(Some(Color::Magenta))),
            (&*KEYWORD_RE, Style::new().fg_color(Some(Color::Cyan)).modifier(Modifier::BOLD)),
            (&*BOOLEAN_NIL_RE, Style::new().fg_color(Some(Color::Yellow))),
            (&*PARENS_RE, Style::new().fg_color(Some(Color::Blue))),
            // Symbol RE is broad, so it's last.
            // It might incorrectly style parts of other tokens if not careful,
            // but rustyline processes char by char with `highlight_char`.
            // However, this `highlight` method gives more control.
        ];

        while current_pos < line.len() {
            let mut found_match = false;
            for (regex, style) in &tokens_with_styles {
                if let Some(mat) = regex.find_at(line, current_pos) {
                    if mat.start() == current_pos { // Ensure match is at the current position
                        // Append unstyled text before the match if any (should not happen if mat.start() == current_pos)
                        // styled_text.append(StyledText::plain(&line[current_pos..mat.start()]));
                        styled_text.append(StyledText::styled(line[mat.start()..mat.end()].to_string(), *style));
                        current_pos = mat.end();
                        found_match = true;
                        break;
                    }
                }
            }

            if !found_match {
                // No specific token found, append as plain text (or symbol style)
                // For simplicity, advance by one char and style it as plain or default symbol.
                // A more sophisticated approach would be to find the next match of *any* token.
                let end_of_plain = line[current_pos..].chars().next().map_or(line.len(), |c| current_pos + c.len_utf8());
                styled_text.append(StyledText::plain(line[current_pos..end_of_plain].to_string()));
                current_pos = end_of_plain;
            }
        }
        // The MatchingBracketHighlighter logic is complex to merge here directly.
        // It's simpler to rely on its effect via highlight_char if that's sufficient,
        // or to implement custom bracket matching within this loop if needed.
        // For now, we return the syntax highlighting from the regexes.
        Owned(styled_text)
    }

    fn highlight_char(&self, line: &str, pos: usize, _forced: bool) -> bool {
        // Delegate to MatchingBracketHighlighter for char-level highlighting (e.g., cursor on bracket)
        self.matching_bracket_highlighter.highlight_char(line, pos, _forced)
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
    // Use the alias CompletionCandidate which refers to rustyline::completion::Candidate struct
    type Candidate = CompletionCandidate; 

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
    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, StyledText> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize, forced: bool) -> bool {
        self.highlighter.highlight_char(line, pos, forced)
    }
}

impl Validator for ReplHelper {
    fn validate(
        &self,
        _ctx: &mut rustyline::validate::ValidationContext,
    ) -> Result<rustyline::validate::ValidationResult, ReadlineError> {
        Ok(rustyline::validate::ValidationResult::Valid(None)) // No-op validation
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
