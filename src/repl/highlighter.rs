use lazy_static::lazy_static;
use regex::Regex;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
// Validator related imports are unused for now, ReplHelper derive will provide default.
// use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::completion::{Completer, CompletionCandidate};
use rustyline::hint::Hinter;
use rustyline::history::History;
use rustyline::Helper as RustylineHelperTrait; // Alias for clarity if needed, or use directly
use rustyline_derive::Helper as RustylineHelperMacro;
use std::borrow::Cow::{self, Owned}; // Removed Borrowed as it's not used after fix
use rustyline::style::{Style, Color, Modifier}; // Corrected import for Style, Color, Modifier
use rustyline::styled_text::StyledText; // Corrected import for StyledText

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


// The RustylineHelperMacro derive should provide Completer, Hinter, Validator defaults.
// We only need to explicitly implement Highlighter for ReplHelper if the derive doesn't
// automatically pick up the highlighter field.
// However, the common pattern with rustyline-derive is that `#[derive(Helper)]`
// expects the struct to have fields named `completer`, `hinter`, `highlighter`, `validator`
// or it provides defaults if these fields are not present and the derive handles it.
// Let's ensure ReplHelper correctly uses LispHighlighter.

#[derive(RustylineHelperMacro)]
pub struct ReplHelper {
    highlighter: LispHighlighter,
    // If we wanted to customize other parts, we'd add fields like:
    // completer: MyCompleter,
    // hinter: MyHinter,
    // validator: MyValidator,
}

impl ReplHelper {
    pub fn new() -> Self {
        Self {
            highlighter: LispHighlighter::default(),
        }
    }
}

// The `rustyline-derive::Helper` macro should generate the necessary
// trait implementations for `Completer`, `Hinter`, `Validator` (as no-ops or defaults)
// and delegate `Highlighter` calls to the `highlighter` field if it exists.
// If not, we would need to implement them manually as shown in the commented block.
// For now, we rely on the derive macro.

impl Default for ReplHelper {
    fn default() -> Self {
        Self::new()
    }
}
