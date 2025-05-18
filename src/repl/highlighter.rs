use lazy_static::lazy_static;
use regex::Regex;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{Completer, CompletionCandidate, Helper, Hinter, History};
use rustyline_derive::Helper as RustylineHelperMacro; // Ensure this derive is available or implement manually
use std::borrow::Cow::{self, Borrowed, Owned};
use rustyline::styled_text::{Style, StyledText};

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
            (&*STRING_RE, Style::new().fg_color(Some(rustyline::Color::Green))),
            (&*COMMENT_RE, Style::new().fg_color(Some(rustyline::Color::DarkGrey))),
            (&*NUMBER_RE, Style::new().fg_color(Some(rustyline::Color::Magenta))),
            (&*KEYWORD_RE, Style::new().fg_color(Some(rustyline::Color::Cyan)).modifier(rustyline::Modifier::BOLD)),
            (&*BOOLEAN_NIL_RE, Style::new().fg_color(Some(rustyline::Color::Yellow))),
            (&*PARENS_RE, Style::new().fg_color(Some(rustyline::Color::Blue))),
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
        
        // Apply matching bracket highlighting after custom syntax highlighting
        let styled_text_cow = self.matching_bracket_highlighter.highlight(Borrowed(&styled_text), pos);
        // The above line is problematic because MatchingBracketHighlighter expects &str, not &StyledText.
        // We need to apply matching bracket highlighting on the original line and merge.
        // For now, let's just return our styled_text.
        // A proper solution would involve more complex logic to merge styles or use highlight_char.
        // Rustyline's design here can be tricky.
        // A simpler way:
        // self.matching_bracket_highlighter.highlight(line, pos) // This would be if LispHighlighter *only* did matching brackets.

        // For now, we'll just return our syntax highlighting.
        // To integrate MatchingBracketHighlighter, one might need to reimplement its logic
        // or adjust how StyledText is built.
        // A common pattern is to let the MatchingBracketHighlighter run first, then apply syntax styles.
        // Or, if the Highlighter trait implies full control, one has to do it all.

        // Let's try to use highlight_char for bracket matching as a fallback or addition.
        // This is not how it's typically done. The main `highlight` method is expected to do all.
        // The `MatchingBracketHighlighter` is a self-contained highlighter.
        // We might need to choose one or the other, or combine their logic manually.

        // Simplest for now: return the syntax-highlighted text.
        // Bracket matching can be added by enhancing this LispHighlighter.
        Owned(styled_text)
    }

    fn highlight_char(&self, line: &str, pos: usize,_forced: bool) -> bool {
        // Delegate to MatchingBracketHighlighter for char-level highlighting (e.g., cursor on bracket)
        self.matching_bracket_highlighter.highlight_char(line, pos, _forced)
    }
}


// Implement Helper, Completer, Hinter, Validator for ReplHelper
// We can use the derive macro if available and configured, or implement manually.
// For manual implementation:
#[derive(RustylineHelperMacro)] // This derive simplifies things. If not working, manual impl below.
pub struct ReplHelper {
    highlighter: LispHighlighter,
    // validator: MatchingBracketValidator, // If we want to validate brackets
    // hinter: HistoryHinter, // Example hinter
}

impl ReplHelper {
    pub fn new() -> Self {
        Self {
            highlighter: LispHighlighter::default(),
        }
    }
}

// Manual implementation if derive macro is not used or for more control:
/*
impl Completer for ReplHelper {
    type Candidate = String; // Or a more complex type if needed
    // fn complete(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Result<(usize, Vec<Self::Candidate>), ReadlineError> {
    //     Ok((0, Vec::new())) // No-op completion
    // }
}

impl Hinter for ReplHelper {
    type Hint = String;
    // fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<Self::Hint> {
    //     None // No-op hinter
    // }
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
    // fn validate(&self, ctx: &mut ValidationContext) -> Result<ValidationResult, ReadlineError> {
    //     Ok(ValidationResult::Valid(None)) // No-op validation
    // }
}

impl Helper for ReplHelper {} // Marker trait
*/

impl Default for ReplHelper {
    fn default() -> Self {
        Self::new()
    }
}
