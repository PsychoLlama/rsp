
use nom::{
    character::complete::multispace0, // For handling whitespace, removed multispace1
    // combinator::map, // Removed as Parser::map method is used
    branch::alt,                                     // For trying multiple parsers
    bytes::complete::tag,                            // For matching literal strings
    character::complete::{satisfy, multispace1},     // For character-level parsing & whitespace
    combinator::{map, recognize},                    // For transforming and recognizing parser output
    multi::{many0, separated_list0},                 // For repeating parsers
    number::complete::double,                        // For parsing f64 numbers
    sequence::{delimited, pair},                      // For sequencing parsers
    IResult,
    Parser, // Import the Parser trait to use its methods like .map() and .parse()
};
use tracing::trace; // For logging parser activity

use crate::ast::Expr; // Assuming your AST expressions are in ast::Expr

// Helper to consume whitespace around a parser (UNUSED after refactor, kept for now if needed elsewhere)
// Takes a parser `inner` and returns a new parser that consumes whitespace around `inner`.
// fn ws<'a, P, O, E>(inner: P) -> impl nom::Parser<&'a str, Output = O, Error = E>
// where
//     P: nom::Parser<&'a str, Output = O, Error = E>,
//     E: nom::error::ParseError<&'a str>,
// {
//     delimited(multispace0, inner, multispace0)
// }

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

// Parses a symbol - raw token.
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn parse_symbol_raw(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse symbol");

    // Define characters allowed to start a symbol
    let initial_char = satisfy(|c: char| {
        c.is_alphabetic() || "!$%&*/:<=>?@^_~+-".contains(c)
    });

    // Define characters allowed in subsequent parts of a symbol
    let subsequent_char = satisfy(|c: char| {
        c.is_alphanumeric() || "!$%&*/:<=>?@^_~+-.#".contains(c)
    });

    // A symbol is an initial character followed by zero or more subsequent characters.
    // `recognize` captures the consumed input slice.
    let symbol_str_parser = recognize(pair(initial_char, many0(subsequent_char)));

    symbol_str_parser
        .map(|s: &str| Expr::Symbol(s.to_string()))
        .parse(input)
}

// Parses a list of expressions e.g. (a b c) or (+ 1 2) - raw token (parens are part of token).
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn parse_list_raw(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse raw list token");
    delimited(
        tag("("), // Matches the opening parenthesis
        // `separated_list0` parses zero or more occurrences of `parse_expr`,
        // separated by one or more whitespace characters.
        separated_list0(
            multispace1, // The separator between elements
            parse_expr   // The public, whitespace-aware parser for each element
        ),
        tag(")")  // Matches the closing parenthesis
    )
    .map(Expr::List) // Converts the Vec<Expr> from separated_list0 into Expr::List
    .parse(input)
}

// Core parser for any single expression type, without leading/trailing whitespace.
// This is used by parse_expr and recursively by parse_list_raw.
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn parse_expr_core(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse core expression token");
    alt((
        parse_number_raw,
        parse_true_raw,
        parse_false_raw,
        parse_nil_raw,
        parse_list_raw,   // Try parsing a list before a general symbol
        parse_symbol_raw,
    ))
    .parse(input)
}

// Top-level parser function for a single expression.
// Handles leading whitespace before parsing the core expression.
// Also handles trailing whitespace after the expression (implicitly, as parse_expr_core consumes what it needs
// and leaves the rest; if this is used in `separated_list0`, the separator handles intermediate space).
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
pub fn parse_expr(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse expression (with whitespace handling)");
    // Each expression unit handles its own leading whitespace.
    // Trailing whitespace is handled by the context (e.g. separator in a list, or end of input).
    preceded(multispace0, parse_expr_core).parse(input)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::setup_tracing;

    #[test]
    fn test_parse_simple_number() {
        setup_tracing();
        let result = parse_expr("123");
        assert_eq!(result, Ok(("", Expr::Number(123.0))));
    }

    #[test]
    fn test_parse_number_with_leading_whitespace() {
        setup_tracing();
        let result = parse_expr("  456");
        assert_eq!(result, Ok(("", Expr::Number(456.0))));
    }

    #[test]
    fn test_parse_number_with_trailing_whitespace() {
        setup_tracing();
        let result = parse_expr("789  ");
        assert_eq!(result, Ok(("", Expr::Number(789.0))));
    }

    #[test]
    fn test_parse_number_with_both_whitespace() {
        setup_tracing();
        let result = parse_expr("  123.45  ");
        assert_eq!(result, Ok(("", Expr::Number(123.45))));
    }

    #[test]
    fn test_parse_negative_number() {
        setup_tracing();
        let result = parse_expr("-10.5");
        assert_eq!(result, Ok(("", Expr::Number(-10.5))));
    }

    #[test]
    fn test_parse_number_with_plus_sign() {
        setup_tracing();
        // nom's `double` parser handles optional leading `+` or `-`.
        let result = parse_expr("+77");
        assert_eq!(result, Ok(("", Expr::Number(77.0))));
    }
    
    #[test]
    fn test_parse_number_scientific_notation() {
        setup_tracing();
        let result = parse_expr("1.23e-4");
        assert_eq!(result, Ok(("", Expr::Number(0.000123))));
        let result_caps = parse_expr("  3.14E5  ");
        assert_eq!(result_caps, Ok(("", Expr::Number(314000.0))));
    }

    #[test]
    fn test_parse_number_leaves_remaining_input() {
        setup_tracing();
        let result = parse_expr("123 abc");
        assert_eq!(result, Ok(("abc", Expr::Number(123.0)))); // Corrected: ws consumes the space after the number
    }
    
    #[test]
    fn test_parse_number_leaves_remaining_input_no_trailing_ws_for_number() {
        setup_tracing();
        let result = parse_expr("123abc"); // No space after number
        assert_eq!(result, Ok(("abc", Expr::Number(123.0))));
    }


    #[test]
    fn test_parse_not_a_number() {
        setup_tracing();
        let result = parse_expr("abc");
        // "abc" is not a number, bool, or nil, so it should be parsed as a symbol.
        assert_eq!(result, Ok(("", Expr::Symbol("abc".to_string()))), "Should parse 'abc' as a symbol. Got: {:?}", result);
    }

    #[test]
    fn test_parse_empty_input() {
        setup_tracing();
        let result = parse_expr("");
        assert!(result.is_err(), "Should fail to parse empty string. Got: {:?}", result);
    }

    #[test]
    fn test_parse_only_whitespace() {
        setup_tracing();
        let result = parse_expr("   ");
         assert!(result.is_err(), "Should fail to parse only whitespace. Got: {:?}", result);
    }

    // Tests for true, false, nil literals
    #[test]
    fn test_parse_true_literal() {
        setup_tracing();
        assert_eq!(parse_expr("true"), Ok(("", Expr::Bool(true))));
        assert_eq!(parse_expr("  true  "), Ok(("", Expr::Bool(true))));
    }

    #[test]
    fn test_parse_false_literal() {
        setup_tracing();
        assert_eq!(parse_expr("false"), Ok(("", Expr::Bool(false))));
        assert_eq!(parse_expr("  false  "), Ok(("", Expr::Bool(false))));
    }

    #[test]
    fn test_parse_nil_literal() {
        setup_tracing();
        assert_eq!(parse_expr("nil"), Ok(("", Expr::Nil)));
        assert_eq!(parse_expr("  nil  "), Ok(("", Expr::Nil)));
    }

    // Tests for symbols
    #[test]
    fn test_parse_simple_symbol() {
        setup_tracing();
        assert_eq!(parse_expr("foo"), Ok(("", Expr::Symbol("foo".to_string()))));
        assert_eq!(parse_expr("  bar  "), Ok(("", Expr::Symbol("bar".to_string()))));
    }

    #[test]
    fn test_parse_symbol_with_hyphen() {
        setup_tracing();
        assert_eq!(parse_expr("my-variable"), Ok(("", Expr::Symbol("my-variable".to_string()))));
    }

    #[test]
    fn test_parse_symbol_with_numbers() {
        setup_tracing();
        assert_eq!(parse_expr("var123"), Ok(("", Expr::Symbol("var123".to_string()))));
    }
    
    #[test]
    fn test_parse_symbol_with_question_mark() {
        setup_tracing();
        assert_eq!(parse_expr("list?"), Ok(("", Expr::Symbol("list?".to_string()))));
    }
    
    #[test]
    fn test_parse_symbol_with_special_chars() {
        setup_tracing();
        assert_eq!(parse_expr("+"), Ok(("", Expr::Symbol("+".to_string()))));
        assert_eq!(parse_expr("-"), Ok(("", Expr::Symbol("-".to_string()))));
        assert_eq!(parse_expr("*"), Ok(("", Expr::Symbol("*".to_string()))));
        assert_eq!(parse_expr("/"), Ok(("", Expr::Symbol("/".to_string()))));
        assert_eq!(parse_expr("="), Ok(("", Expr::Symbol("=".to_string()))));
        assert_eq!(parse_expr("<="), Ok(("", Expr::Symbol("<=".to_string()))));
    }

    #[test]
    fn test_parse_symbol_is_not_number() {
        setup_tracing();
        // "123" should be parsed by parse_number, not parse_symbol
        let result = parse_expr("123");
        assert_eq!(result, Ok(("", Expr::Number(123.0))));
        // Ensure it's not misinterpreted as a symbol if parse_number was absent
        // This is implicitly tested by alt order, but let's be clear.
        // If we called parse_symbol directly:
        // assert!(parse_symbol("123").is_err(), "Symbol parser should not parse '123'");
        // Current symbol definition doesn't allow starting with a digit unless it's part of a special char like `+`
        // So parse_symbol("123") would fail anyway.
    }

    #[test]
    fn test_parse_symbol_keywords_as_symbols() {
        setup_tracing();
        // Keywords for special forms should parse as symbols
        assert_eq!(parse_expr("let"), Ok(("", Expr::Symbol("let".to_string()))));
        assert_eq!(parse_expr("if"), Ok(("", Expr::Symbol("if".to_string()))));
        assert_eq!(parse_expr("quote"), Ok(("", Expr::Symbol("quote".to_string()))));
        assert_eq!(parse_expr("fn"), Ok(("", Expr::Symbol("fn".to_string()))));
    }

    #[test]
    fn test_parse_symbol_leaves_remaining_input() {
        setup_tracing();
        assert_eq!(parse_expr("symbol-name rest"), Ok(("rest", Expr::Symbol("symbol-name".to_string()))));
        assert_eq!(parse_expr("  symbol-name   rest"), Ok(("rest", Expr::Symbol("symbol-name".to_string()))));
    }
    
    #[test]
    fn test_parse_true_leaves_remaining_input() {
        setup_tracing();
        assert_eq!(parse_expr("true rest"), Ok(("rest", Expr::Bool(true))));
    }

    #[test]
    fn test_parse_symbol_starting_with_dot_if_allowed() {
        // Current symbol definition: initial_char does not include '.', subsequent_char does.
        // So ".foo" would not parse. If initial_char included '.', this test would be relevant.
        // For now, this behavior is as expected (error).
        setup_tracing();
        let result = parse_expr(".foo");
        assert!(result.is_err(), "Symbol starting with '.' should fail with current rules: {:?}", result);
    }
    
    #[test]
    fn test_parse_symbol_with_dot_allowed_internally() {
        setup_tracing();
        assert_eq!(parse_expr("foo.bar"), Ok(("", Expr::Symbol("foo.bar".to_string()))));
    }

    #[test]
    fn test_parse_symbol_cannot_be_just_dots_if_not_special() {
        // ".." or "..." might be special in some Lisps, but our current rule:
        // initial_char does not include '.', subsequent_char does.
        // So ".." would fail because first '.' is not an initial_char.
        // If initial_char allowed '.', then ".." would be `initial='.'`, `subsequent=['.']`.
        setup_tracing();
        // A single dot '.' is not a valid symbol by current rules (not in initial_char set).
        assert!(parse_expr(".").is_err(), "Single dot symbol should fail with current rules. Got: {:?}", parse_expr("."));
        assert!(parse_expr("..").is_err(), "Double dot symbol should fail with current rules. Got: {:?}", parse_expr(".."));
        assert!(parse_expr("...").is_err(), "Triple dot symbol should fail with current rules. Got: {:?}", parse_expr("..."));
    }

    // Tests for lists
    #[test]
    fn test_parse_empty_list() {
        setup_tracing();
        assert_eq!(parse_expr("()"), Ok(("", Expr::List(vec![]))));
        assert_eq!(parse_expr(" ( ) "), Ok(("", Expr::List(vec![]))));
    }

    #[test]
    fn test_parse_list_with_one_number() {
        setup_tracing();
        assert_eq!(parse_expr("(1)"), Ok(("", Expr::List(vec![Expr::Number(1.0)]))));
        assert_eq!(parse_expr(" ( 1 ) "), Ok(("", Expr::List(vec![Expr::Number(1.0)]))));
    }

    #[test]
    fn test_parse_list_with_multiple_numbers() {
        setup_tracing();
        assert_eq!(
            parse_expr("(1 2 3)"),
            Ok((
                "",
                Expr::List(vec![Expr::Number(1.0), Expr::Number(2.0), Expr::Number(3.0)])
            ))
        );
        assert_eq!(
            parse_expr(" (  1   2   3  ) "),
            Ok((
                "",
                Expr::List(vec![Expr::Number(1.0), Expr::Number(2.0), Expr::Number(3.0)])
            ))
        );
    }

    #[test]
    fn test_parse_list_with_symbols() {
        setup_tracing();
        assert_eq!(
            parse_expr("(a b c)"),
            Ok((
                "",
                Expr::List(vec![
                    Expr::Symbol("a".to_string()),
                    Expr::Symbol("b".to_string()),
                    Expr::Symbol("c".to_string())
                ])
            ))
        );
    }

    #[test]
    fn test_parse_list_with_mixed_types() {
        setup_tracing();
        assert_eq!(
            parse_expr("(+ 1 foo)"),
            Ok((
                "",
                Expr::List(vec![
                    Expr::Symbol("+".to_string()),
                    Expr::Number(1.0),
                    Expr::Symbol("foo".to_string())
                ])
            ))
        );
    }

    #[test]
    fn test_parse_nested_empty_list() {
        setup_tracing();
        assert_eq!(
            parse_expr("(())"),
            Ok(("", Expr::List(vec![Expr::List(vec![])])))
        );
        assert_eq!(
            parse_expr("( ( ) )"), // With spaces
            Ok(("", Expr::List(vec![Expr::List(vec![])])))
        );
    }

    #[test]
    fn test_parse_nested_list() {
        setup_tracing();
        assert_eq!(
            parse_expr("(a (b) c)"),
            Ok((
                "",
                Expr::List(vec![
                    Expr::Symbol("a".to_string()),
                    Expr::List(vec![Expr::Symbol("b".to_string())]),
                    Expr::Symbol("c".to_string())
                ])
            ))
        );
    }
    
    #[test]
    fn test_parse_deeply_nested_list() {
        setup_tracing();
        let input = "(a (b (c (d) e) f) g)";
        let expected = Expr::List(vec![
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
        ]);
        assert_eq!(parse_expr(input), Ok(("", expected)));
    }


    #[test]
    fn test_parse_list_leaves_remaining_input() {
        setup_tracing();
        assert_eq!(
            parse_expr("(a b) c"),
            Ok((
                "c", // Note: ws around list consumes space after ')', so " c" becomes "c"
                Expr::List(vec![
                    Expr::Symbol("a".to_string()),
                    Expr::Symbol("b".to_string())
                ])
            ))
        );
    }

    #[test]
    fn test_parse_list_unmatched_opening_paren() {
        setup_tracing();
        let result = parse_expr("(a b");
        assert!(result.is_err(), "Should fail for unmatched opening parenthesis. Got: {:?}", result);
    }
    
    #[test]
    fn test_parse_list_unmatched_closing_paren() {
        setup_tracing();
        // This case is tricky. "a b)" might be parsed as symbol "a", leaving "b)"
        // Or, if `parse_expr` is part of a larger structure expecting balanced forms,
        // the error might be caught at a higher level.
        // For `parse_expr` itself, it would parse `a` and leave `b)`.
        // If we parse `(a b))`, it should parse `(a b)` and leave `)`.
        let result = parse_expr("(a b))");
         assert_eq!(
            result,
            Ok((
                ")", 
                Expr::List(vec![
                    Expr::Symbol("a".to_string()),
                    Expr::Symbol("b".to_string())
                ])
            ))
        );

        let result_just_paren = parse_expr(")");
        assert!(result_just_paren.is_err(), "Should fail for stray closing parenthesis. Got: {:?}", result_just_paren);
    }

    #[test]
    fn test_parse_list_no_space_between_elements() {
        setup_tracing();
        // "(ab)" should parse as a list containing one symbol "ab"
        // because `multispace1` is the separator.
        assert_eq!(
            parse_expr("(ab)"),
            Ok(("", Expr::List(vec![Expr::Symbol("ab".to_string())])))
        );
        // "(a b)" is fine
        assert_eq!(
            parse_expr("(a b)"),
            Ok(("", Expr::List(vec![Expr::Symbol("a".to_string()), Expr::Symbol("b".to_string())])))
        );
        // "(1-2)" should be a list with one symbol "1-2" if symbols can start with numbers when followed by non-numbers
        // Current symbol rule: initial_char cannot be a digit. So "1-2" is not a symbol.
        // It's also not a number. So `parse_expr("1-2")` would fail.
        // Thus `(1-2)` should fail to parse its element.
        let result = parse_expr("(1-2)");
        assert!(result.is_err(), "Parsing (1-2) should fail as 1-2 is not a valid expr. Got: {:?}", result);

        // "(+1)" should be a list with one symbol "+1" if symbols can be like that.
        // `+` is an initial_char, `1` is a subsequent_char. So `+1` is a symbol.
         assert_eq!(
            parse_expr("(+1)"),
            Ok(("", Expr::List(vec![Expr::Symbol("+1".to_string())])))
        );
    }
}
