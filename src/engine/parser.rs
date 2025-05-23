use nom::{
    IResult,
    Parser,      // Import the Parser trait to use its methods like .map() and .parse()
    branch::alt, // For trying multiple parsers
    bytes::complete::{is_not, tag}, // Removed escaped_transform
    character::complete::{char, multispace1, not_line_ending, satisfy}, // Added not_line_ending, Removed none_of
    combinator::{opt, recognize, verify},                               // Added opt, Added verify
    multi::{fold_many0, many0, many1, separated_list0}, // Added fold_many0 and many1
    number::complete::double,                           // For parsing f64 numbers
    sequence::{delimited, pair, preceded, terminated},  // For sequencing parsers
};
use tracing::trace; // For logging parser activity

use crate::engine::ast::Expr; // Assuming your AST expressions are in ast::Expr

// Parses a single comment line (from ';' to EOL, not including EOL itself).
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn parse_comment_line(input: &str) -> IResult<&str, &str> {
    trace!("Attempting to parse comment line");
    recognize(pair(char(';'), not_line_ending)).parse(input)
}

// Consumes zero or more whitespace characters or full-line comments.
// Each "ignored item" is either a chunk of whitespace1 or a comment line.
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn space_or_comment0(input: &str) -> IResult<&str, &str> {
    trace!("Attempting to parse zero or more spaces/comments");
    recognize(many0(alt((
        multispace1, // Consumes whitespace including newlines
        parse_comment_line,
    ))))
    .parse(input)
}

// Consumes one or more whitespace characters or full-line comments.
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn space_or_comment1(input: &str) -> IResult<&str, &str> {
    trace!("Attempting to parse one or more spaces/comments");
    recognize(many1(alt((multispace1, parse_comment_line)))).parse(input)
}

// Parses a number (f64) into an Expr::Number - raw token, no surrounding whitespace handling.
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn parse_number_raw(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse raw number token");
    double.map(Expr::Number).parse(input)
}

// Parses the keyword "true" into an Expr::Bool(true) - raw token.
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn parse_true_raw(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse raw 'true' literal token");
    tag("true").map(|_| Expr::Bool(true)).parse(input)
}

// Parses the keyword "false" into an Expr::Bool(false) - raw token.
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn parse_false_raw(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse raw 'false' literal token");
    tag("false").map(|_| Expr::Bool(false)).parse(input)
}

// Parses the keyword "nil" into an Expr::Nil - raw token.
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn parse_nil_raw(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse raw 'nil' literal token");
    tag("nil").map(|_| Expr::Nil).parse(input)
}

// Helper: Parse a non-empty sequence of unescaped characters.
// Ensures that it consumes at least one character if it matches.
fn parse_unescaped_char_sequence(input: &str) -> IResult<&str, &str> {
    verify(is_not("\"\\"), |s: &str| !s.is_empty()).parse(input)
}

// Helper: Parse an escaped character and return it as a String.
fn parse_escaped_char(input: &str) -> IResult<&str, String> {
    preceded(
        char('\\'),
        alt((
            tag("\"").map(|_| "\"".to_string()),
            tag("\\").map(|_| "\\".to_string()),
            tag("n").map(|_| "\n".to_string()),
            tag("r").map(|_| "\r".to_string()),
            tag("t").map(|_| "\t".to_string()),
            // Add other escapes here if needed, e.g., unicode \uXXXX
        )),
    )
    .parse(input)
}

// Parses a string literal e.g. "hello world" or "escaped \" char" - raw token.
// Handles empty strings "" correctly.
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn parse_string_raw(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse raw string literal token");
    delimited(
        char('"'),
        fold_many0(
            alt((
                // Try to parse a sequence of unescaped characters
                parse_unescaped_char_sequence.map(|s: &str| s.to_string()),
                // Try to parse a single escaped character
                parse_escaped_char,
            )),
            String::new, // Initial accumulator for the string
            |mut acc, s_fragment| {
                acc.push_str(&s_fragment);
                acc
            },
        ),
        char('"'),
    )
    .map(Expr::String) // Map the accumulated String to Expr::String
    .parse(input)
}

// Parses a quoted expression e.g., 'foo or '(1 2) - raw token.
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn parse_quoted_expr_raw(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse raw quoted expression token");
    preceded(
        tag("'"),
        // The expression being quoted can have leading whitespace/comments after the quote character.
        preceded(space_or_comment0, expr_recursive_impl),
    )
    .map(|expr| Expr::List(vec![Expr::Symbol("quote".to_string()), expr]))
    .parse(input)
}

// Parses a symbol - raw token.
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn parse_symbol_raw(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse symbol");

    // Define characters allowed to start a symbol
    let initial_char = satisfy(|c: char| c.is_alphabetic() || "!$%&*/:<=>?@^_~+-".contains(c));

    // Define characters allowed in subsequent parts of a symbol
    let subsequent_char =
        satisfy(|c: char| c.is_alphanumeric() || "!$%&*/:<=>?@^_~+-.#".contains(c));

    // A symbol is an initial character followed by zero or more subsequent characters.
    // `recognize` captures the consumed input slice.
    let symbol_str_parser = recognize(pair(initial_char, many0(subsequent_char)));

    symbol_str_parser
        .map(|s: &str| Expr::Symbol(s.to_string()))
        .parse(input)
}

// Parses a list of expressions e.g. (a b c) or (+ 1 2) - raw token (parens are part of token).
// This function is recursive with expr_recursive_impl.
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn list_raw(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse raw list token");
    delimited(
        // Consume (
        tag("("),
        // Consume elements separated by space_or_comment1.
        // Also consume any space/comments before the closing parenthesis.
        terminated(
            separated_list0(
                space_or_comment1, // Separator: one or more spaces/comments
                // Element parser: consumes leading spaces/comments, then one core expression
                preceded(space_or_comment0, expr_recursive_impl),
            ),
            space_or_comment0, // Consume trailing spaces/comments before the closing parenthesis
        ),
        // Consume )
        tag(")"),
    )
    .map(Expr::List)
    .parse(input)
}

// Core recursive parser for any single expression type (atom or list), without surrounding whitespace.
// This is the heart of the recursive descent.
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn expr_recursive_impl(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse core expression token (recursive_impl)");
    alt((
        parse_number_raw,
        parse_true_raw,
        parse_false_raw,
        parse_nil_raw,
        parse_quoted_expr_raw, // Added for 'expr syntax
        parse_string_raw,
        list_raw,
        parse_symbol_raw,
    ))
    .parse(input)
}

// Top-level parser function for a single expression.
// Handles leading AND trailing whitespace/comments.
// Returns Option<Expr> to indicate if an actual expression was found.
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
pub fn parse_expr(input: &str) -> IResult<&str, Option<Expr>> {
    trace!(
        "Attempting to parse expression (optional, with surrounding whitespace/comment handling)"
    );
    preceded(
        space_or_comment0, // Consume leading spaces/comments
        opt(terminated(
            // The core expression is optional
            expr_recursive_impl,
            space_or_comment0, // Consume trailing spaces/comments *after* the expression
        )),
    )
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::init_test_logging;

    #[test]
    fn test_parse_simple_number() {
        init_test_logging();
        let result = parse_expr("123");
        assert_eq!(result, Ok(("", Some(Expr::Number(123.0)))));
    }

    #[test]
    fn test_parse_number_with_leading_whitespace() {
        init_test_logging();
        let result = parse_expr("  456");
        assert_eq!(result, Ok(("", Some(Expr::Number(456.0)))));
    }

    #[test]
    fn test_parse_number_with_trailing_whitespace() {
        init_test_logging();
        let result = parse_expr("789  ");
        assert_eq!(result, Ok(("", Some(Expr::Number(789.0)))));
    }

    #[test]
    fn test_parse_number_with_both_whitespace() {
        init_test_logging();
        let result = parse_expr("  123.45  ");
        assert_eq!(result, Ok(("", Some(Expr::Number(123.45)))));
    }

    #[test]
    fn test_parse_negative_number() {
        init_test_logging();
        let result = parse_expr("-10.5");
        assert_eq!(result, Ok(("", Some(Expr::Number(-10.5)))));
    }

    #[test]
    fn test_parse_number_with_plus_sign() {
        init_test_logging();
        // nom's `double` parser handles optional leading `+` or `-`.
        let result = parse_expr("+77");
        assert_eq!(result, Ok(("", Some(Expr::Number(77.0)))));
    }

    #[test]
    fn test_parse_number_scientific_notation() {
        init_test_logging();
        let result = parse_expr("1.23e-4");
        assert_eq!(result, Ok(("", Some(Expr::Number(0.000123)))));
        let result_caps = parse_expr("  3.14E5  ");
        assert_eq!(result_caps, Ok(("", Some(Expr::Number(314000.0)))));
    }

    #[test]
    fn test_parse_number_leaves_remaining_input() {
        init_test_logging();
        let result = parse_expr("123 abc");
        assert_eq!(result, Ok(("abc", Some(Expr::Number(123.0)))));
    }

    #[test]
    fn test_parse_number_leaves_remaining_input_no_trailing_ws_for_number() {
        init_test_logging();
        let result = parse_expr("123abc"); // No space after number
        assert_eq!(result, Ok(("abc", Some(Expr::Number(123.0)))));
    }

    #[test]
    fn test_parse_not_a_number() {
        init_test_logging();
        let result = parse_expr("abc");
        // "abc" is not a number, bool, or nil, so it should be parsed as a symbol.
        assert_eq!(
            result,
            Ok(("", Some(Expr::Symbol("abc".to_string())))),
            "Should parse 'abc' as a symbol. Got: {:?}",
            result
        );
    }

    #[test]
    fn test_parse_empty_input() {
        init_test_logging();
        let result = parse_expr("");
        assert_eq!(
            result,
            Ok(("", None)),
            "Empty string should parse as None. Got: {:?}",
            result
        );
    }

    #[test]
    fn test_parse_only_whitespace() {
        init_test_logging();
        let result = parse_expr("   ");
        assert_eq!(
            result,
            Ok(("", None)),
            "Whitespace-only string should parse as None. Got: {:?}",
            result
        );
    }

    // Tests for true, false, nil literals
    #[test]
    fn test_parse_true_literal() {
        init_test_logging();
        assert_eq!(parse_expr("true"), Ok(("", Some(Expr::Bool(true)))));
        assert_eq!(parse_expr("  true  "), Ok(("", Some(Expr::Bool(true)))));
    }

    #[test]
    fn test_parse_false_literal() {
        init_test_logging();
        assert_eq!(parse_expr("false"), Ok(("", Some(Expr::Bool(false)))));
        assert_eq!(parse_expr("  false  "), Ok(("", Some(Expr::Bool(false)))));
    }

    #[test]
    fn test_parse_nil_literal() {
        init_test_logging();
        assert_eq!(parse_expr("nil"), Ok(("", Some(Expr::Nil))));
        assert_eq!(parse_expr("  nil  "), Ok(("", Some(Expr::Nil))));
    }

    // Tests for symbols
    #[test]
    fn test_parse_simple_symbol() {
        init_test_logging();
        assert_eq!(
            parse_expr("foo"),
            Ok(("", Some(Expr::Symbol("foo".to_string()))))
        );
        assert_eq!(
            parse_expr("  bar  "),
            Ok(("", Some(Expr::Symbol("bar".to_string()))))
        );
    }

    #[test]
    fn test_parse_symbol_with_hyphen() {
        init_test_logging();
        assert_eq!(
            parse_expr("my-variable"),
            Ok(("", Some(Expr::Symbol("my-variable".to_string()))))
        );
    }

    #[test]
    fn test_parse_symbol_with_numbers() {
        init_test_logging();
        assert_eq!(
            parse_expr("var123"),
            Ok(("", Some(Expr::Symbol("var123".to_string()))))
        );
    }

    #[test]
    fn test_parse_symbol_with_question_mark() {
        init_test_logging();
        assert_eq!(
            parse_expr("list?"),
            Ok(("", Some(Expr::Symbol("list?".to_string()))))
        );
    }

    #[test]
    fn test_parse_symbol_with_special_chars() {
        init_test_logging();
        assert_eq!(
            parse_expr("+"),
            Ok(("", Some(Expr::Symbol("+".to_string()))))
        );
        assert_eq!(
            parse_expr("-"),
            Ok(("", Some(Expr::Symbol("-".to_string()))))
        );
        assert_eq!(
            parse_expr("*"),
            Ok(("", Some(Expr::Symbol("*".to_string()))))
        );
        assert_eq!(
            parse_expr("/"),
            Ok(("", Some(Expr::Symbol("/".to_string()))))
        );
        assert_eq!(
            parse_expr("="),
            Ok(("", Some(Expr::Symbol("=".to_string()))))
        );
        assert_eq!(
            parse_expr("<="),
            Ok(("", Some(Expr::Symbol("<=".to_string()))))
        );
    }

    #[test]
    fn test_parse_symbol_is_not_number() {
        init_test_logging();
        let result = parse_expr("123");
        assert_eq!(result, Ok(("", Some(Expr::Number(123.0)))));
    }

    #[test]
    fn test_parse_symbol_keywords_as_symbols() {
        init_test_logging();
        assert_eq!(
            parse_expr("let"),
            Ok(("", Some(Expr::Symbol("let".to_string()))))
        );
        assert_eq!(
            parse_expr("if"),
            Ok(("", Some(Expr::Symbol("if".to_string()))))
        );
        assert_eq!(
            parse_expr("quote"),
            Ok(("", Some(Expr::Symbol("quote".to_string()))))
        );
        assert_eq!(
            parse_expr("fn"),
            Ok(("", Some(Expr::Symbol("fn".to_string()))))
        );
    }

    #[test]
    fn test_parse_symbol_leaves_remaining_input() {
        init_test_logging();
        assert_eq!(
            parse_expr("symbol-name rest"),
            Ok(("rest", Some(Expr::Symbol("symbol-name".to_string()))))
        );
        assert_eq!(
            parse_expr("  symbol-name   rest"),
            Ok(("rest", Some(Expr::Symbol("symbol-name".to_string()))))
        );
    }

    #[test]
    fn test_parse_true_leaves_remaining_input() {
        init_test_logging();
        assert_eq!(
            parse_expr("true rest"),
            Ok(("rest", Some(Expr::Bool(true))))
        );
    }

    #[test]
    fn test_parse_symbol_starting_with_dot_if_allowed() {
        init_test_logging();
        let result = parse_expr(".foo");
        assert_eq!(
            result,
            Ok((".foo", None)),
            "Symbol starting with '.' should not parse as a valid expression: {:?}",
            result
        );
    }

    #[test]
    fn test_parse_symbol_with_dot_allowed_internally() {
        init_test_logging();
        assert_eq!(
            parse_expr("foo.bar"),
            Ok(("", Some(Expr::Symbol("foo.bar".to_string()))))
        );
    }

    #[test]
    fn test_parse_symbol_cannot_be_just_dots_if_not_special() {
        init_test_logging();
        assert_eq!(
            parse_expr("."),
            Ok((".", None)),
            "Single dot should not parse as valid expr. Got: {:?}",
            parse_expr(".")
        );
        assert_eq!(
            parse_expr(".."),
            Ok(("..", None)),
            "Double dot should not parse as valid expr. Got: {:?}",
            parse_expr("..")
        );
        assert_eq!(
            parse_expr("..."),
            Ok(("...", None)),
            "Triple dot should not parse as valid expr. Got: {:?}",
            parse_expr("...")
        );
    }

    // Tests for lists
    #[test]
    fn test_parse_empty_list() {
        init_test_logging();
        assert_eq!(parse_expr("()"), Ok(("", Some(Expr::List(vec![])))));
        assert_eq!(parse_expr(" ( ) "), Ok(("", Some(Expr::List(vec![])))));
    }

    #[test]
    fn test_parse_list_with_one_number() {
        init_test_logging();
        assert_eq!(
            parse_expr("(1)"),
            Ok(("", Some(Expr::List(vec![Expr::Number(1.0)]))))
        );
        assert_eq!(
            parse_expr(" ( 1 ) "),
            Ok(("", Some(Expr::List(vec![Expr::Number(1.0)]))))
        );
    }

    #[test]
    fn test_parse_list_with_multiple_numbers() {
        init_test_logging();
        assert_eq!(
            parse_expr("(1 2 3)"),
            Ok((
                "",
                Some(Expr::List(vec![
                    Expr::Number(1.0),
                    Expr::Number(2.0),
                    Expr::Number(3.0)
                ]))
            ))
        );
        assert_eq!(
            parse_expr(" (  1   2   3  ) "),
            Ok((
                "",
                Some(Expr::List(vec![
                    Expr::Number(1.0),
                    Expr::Number(2.0),
                    Expr::Number(3.0)
                ]))
            ))
        );
    }

    #[test]
    fn test_parse_list_with_symbols() {
        init_test_logging();
        assert_eq!(
            parse_expr("(a b c)"),
            Ok((
                "",
                Some(Expr::List(vec![
                    Expr::Symbol("a".to_string()),
                    Expr::Symbol("b".to_string()),
                    Expr::Symbol("c".to_string())
                ]))
            ))
        );
    }

    #[test]
    fn test_parse_list_with_mixed_types() {
        init_test_logging();
        assert_eq!(
            parse_expr("(+ 1 foo)"),
            Ok((
                "",
                Some(Expr::List(vec![
                    Expr::Symbol("+".to_string()),
                    Expr::Number(1.0),
                    Expr::Symbol("foo".to_string())
                ]))
            ))
        );
    }

    #[test]
    fn test_parse_nested_empty_list() {
        init_test_logging();
        assert_eq!(
            parse_expr("(())"),
            Ok(("", Some(Expr::List(vec![Expr::List(vec![])]))))
        );
        assert_eq!(
            parse_expr("( ( ) )"), // With spaces
            Ok(("", Some(Expr::List(vec![Expr::List(vec![])]))))
        );
    }

    #[test]
    fn test_parse_nested_list() {
        init_test_logging();
        assert_eq!(
            parse_expr("(a (b) c)"),
            Ok((
                "",
                Some(Expr::List(vec![
                    Expr::Symbol("a".to_string()),
                    Expr::List(vec![Expr::Symbol("b".to_string())]),
                    Expr::Symbol("c".to_string())
                ]))
            ))
        );
    }

    #[test]
    fn test_parse_deeply_nested_list() {
        init_test_logging();
        let input = "(a (b (c (d) e) f) g)";
        let expected = Some(Expr::List(vec![
            Expr::Symbol("a".to_string()),
            Expr::List(vec![
                Expr::Symbol("b".to_string()),
                Expr::List(vec![
                    Expr::Symbol("c".to_string()),
                    Expr::List(vec![Expr::Symbol("d".to_string())]),
                    Expr::Symbol("e".to_string()),
                ]),
                Expr::Symbol("f".to_string()),
            ]),
            Expr::Symbol("g".to_string()),
        ]));
        assert_eq!(parse_expr(input), Ok(("", expected)));
    }

    #[test]
    fn test_parse_list_leaves_remaining_input() {
        init_test_logging();
        assert_eq!(
            parse_expr("(a b) c"),
            Ok((
                "c",
                Some(Expr::List(vec![
                    Expr::Symbol("a".to_string()),
                    Expr::Symbol("b".to_string())
                ]))
            ))
        );
    }

    #[test]
    fn test_parse_list_unmatched_opening_paren() {
        init_test_logging();
        let result = parse_expr("(a b");
        assert_eq!(
            result,
            Ok(("(a b", None)),
            "Unmatched opening paren should result in no expr parsed. Got: {:?}",
            result
        );
    }

    #[test]
    fn test_parse_list_unmatched_closing_paren() {
        init_test_logging();
        let result = parse_expr("(a b))");
        assert_eq!(
            result,
            Ok((
                ")",
                Some(Expr::List(vec![
                    Expr::Symbol("a".to_string()),
                    Expr::Symbol("b".to_string())
                ]))
            ))
        );

        let result_just_paren = parse_expr(")");
        assert_eq!(
            result_just_paren,
            Ok((")", None)),
            "Stray closing paren should result in no expr parsed. Got: {:?}",
            result_just_paren
        );
    }

    #[test]
    fn test_parse_list_no_space_between_elements() {
        init_test_logging();
        assert_eq!(
            parse_expr("(ab)"),
            Ok(("", Some(Expr::List(vec![Expr::Symbol("ab".to_string())]))))
        );
        assert_eq!(
            parse_expr("(a b)"),
            Ok((
                "",
                Some(Expr::List(vec![
                    Expr::Symbol("a".to_string()),
                    Expr::Symbol("b".to_string())
                ]))
            ))
        );
        let result = parse_expr("(1-2)");
        assert_eq!(
            result,
            Ok(("(1-2)", None)),
            "Parsing (1-2) should result in no expr. Got: {:?}",
            result
        );

        assert_eq!(
            parse_expr("(+1)"),
            Ok(("", Some(Expr::List(vec![Expr::Number(1.0)]))))
        );
    }

    // Tests for quoted expressions
    #[test]
    fn test_parse_quoted_symbol() {
        init_test_logging();
        assert_eq!(
            parse_expr("'foo"),
            Ok((
                "",
                Some(Expr::List(vec![
                    Expr::Symbol("quote".to_string()),
                    Expr::Symbol("foo".to_string())
                ]))
            ))
        );
        assert_eq!(
            parse_expr("  'bar  "), // With surrounding whitespace
            Ok((
                "",
                Some(Expr::List(vec![
                    Expr::Symbol("quote".to_string()),
                    Expr::Symbol("bar".to_string())
                ]))
            ))
        );
    }

    #[test]
    fn test_parse_quoted_number() {
        init_test_logging();
        assert_eq!(
            parse_expr("'123"),
            Ok((
                "",
                Some(Expr::List(vec![
                    Expr::Symbol("quote".to_string()),
                    Expr::Number(123.0)
                ]))
            ))
        );
    }

    #[test]
    fn test_parse_quoted_string() {
        init_test_logging();
        assert_eq!(
            parse_expr("'\"hello world\""),
            Ok((
                "",
                Some(Expr::List(vec![
                    Expr::Symbol("quote".to_string()),
                    Expr::String("hello world".to_string())
                ]))
            ))
        );
    }

    #[test]
    fn test_parse_quoted_list() {
        init_test_logging();
        assert_eq!(
            parse_expr("'(a b c)"),
            Ok((
                "",
                Some(Expr::List(vec![
                    Expr::Symbol("quote".to_string()),
                    Expr::List(vec![
                        Expr::Symbol("a".to_string()),
                        Expr::Symbol("b".to_string()),
                        Expr::Symbol("c".to_string())
                    ])
                ]))
            ))
        );
    }

    #[test]
    fn test_parse_quoted_empty_list() {
        init_test_logging();
        assert_eq!(
            parse_expr("'()"),
            Ok((
                "",
                Some(Expr::List(vec![
                    Expr::Symbol("quote".to_string()),
                    Expr::List(vec![])
                ]))
            ))
        );
    }

    #[test]
    fn test_parse_double_quote() {
        init_test_logging();
        // ''foo  is (quote (quote foo))
        assert_eq!(
            parse_expr("''foo"),
            Ok((
                "",
                Some(Expr::List(vec![
                    Expr::Symbol("quote".to_string()),
                    Expr::List(vec![
                        Expr::Symbol("quote".to_string()),
                        Expr::Symbol("foo".to_string())
                    ])
                ]))
            ))
        );
    }

    #[test]
    fn test_parse_quote_with_internal_whitespace() {
        init_test_logging();
        // '  foo should parse as (quote foo)
        assert_eq!(
            parse_expr("'  foo"),
            Ok((
                "",
                Some(Expr::List(vec![
                    Expr::Symbol("quote".to_string()),
                    Expr::Symbol("foo".to_string())
                ]))
            ))
        );
        // '  (a b) should parse as (quote (a b))
        assert_eq!(
            parse_expr("'  (a b)"),
            Ok((
                "",
                Some(Expr::List(vec![
                    Expr::Symbol("quote".to_string()),
                    Expr::List(vec![
                        Expr::Symbol("a".to_string()),
                        Expr::Symbol("b".to_string())
                    ])
                ]))
            ))
        );
    }

    #[test]
    fn test_parse_quoted_list_with_quoted_element() {
        init_test_logging();
        // '(a 'b c) should parse as (quote (a (quote b) c))
        let expected = Some(Expr::List(vec![
            Expr::Symbol("quote".to_string()),
            Expr::List(vec![
                Expr::Symbol("a".to_string()),
                Expr::List(vec![
                    Expr::Symbol("quote".to_string()),
                    Expr::Symbol("b".to_string()),
                ]),
                Expr::Symbol("c".to_string()),
            ]),
        ]));
        assert_eq!(parse_expr("'(a 'b c)"), Ok(("", expected)));
    }

    #[test]
    fn test_parse_quote_leaves_remaining_input() {
        init_test_logging();
        assert_eq!(
            parse_expr("'foo bar"),
            Ok((
                "bar",
                Some(Expr::List(vec![
                    Expr::Symbol("quote".to_string()),
                    Expr::Symbol("foo".to_string())
                ]))
            ))
        );
    }
}
