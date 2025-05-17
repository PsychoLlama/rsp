
use nom::{
    character::complete::multispace0, // For handling whitespace, removed multispace1
    // combinator::map, // Removed as Parser::map method is used
    branch::alt,                                     // For trying multiple parsers
    bytes::complete::tag,                            // For matching literal strings
    character::complete::{alphanumeric0, satisfy},   // For character-level parsing
    combinator::{map, recognize},                    // For transforming and recognizing parser output
    multi::many0,                                    // For repeating a parser zero or more times
    number::complete::double,                        // For parsing f64 numbers
    sequence::{delimited, pair},                      // For sequencing parsers
    IResult,
    Parser, // Import the Parser trait to use its methods like .map() and .parse()
};
use tracing::trace; // For logging parser activity

use crate::ast::Expr; // Assuming your AST expressions are in ast::Expr

// Helper to consume whitespace around a parser
// Takes a parser `inner` and returns a new parser that consumes whitespace around `inner`.
fn ws<'a, P, O, E>(inner: P) -> impl nom::Parser<&'a str, Output = O, Error = E>
where
    P: nom::Parser<&'a str, Output = O, Error = E>,
    E: nom::error::ParseError<&'a str>,
{
    delimited(multispace0, inner, multispace0)
}

// Parses a number (f64) into an Expr::Number
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn parse_number(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse number");
    // ws(double) returns a parser.
    // .map(Expr::Number) is the Parser trait's map method, returning a new parser.
    // .parse(input) executes the parser.
    (ws(double).map(Expr::Number)).parse(input)
}

// Parses the keyword "true" into an Expr::Bool(true)
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn parse_true(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse 'true' literal");
    map(ws(tag("true")), |_| Expr::Bool(true)).parse(input)
}

// Parses the keyword "false" into an Expr::Bool(false)
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn parse_false(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse 'false' literal");
    map(ws(tag("false")), |_| Expr::Bool(false)).parse(input)
}

// Parses the keyword "nil" into an Expr::Nil
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn parse_nil(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse 'nil' literal");
    map(ws(tag("nil")), |_| Expr::Nil).parse(input)
}

// Parses a symbol
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
fn parse_symbol(input: &str) -> IResult<&str, Expr> {
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

    map(ws(symbol_str_parser), |s: &str| {
        Expr::Symbol(s.to_string())
    })
    .parse(input)
}

// Top-level parser function for a single expression
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
pub fn parse_expr(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse expression");
    alt((
        parse_number,
        parse_true,
        parse_false,
        parse_nil,
        parse_symbol,
        // TODO: parse_list will be added here later
    ))(input)
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
        assert!(result.is_err(), "Should fail to parse 'abc' as a number. Got: {:?}", result);
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
        assert!(parse_expr(".").is_ok(), "Single dot symbol should parse"); // `.` is a valid symbol
        assert!(parse_expr("..").is_err(), "Double dot symbol should fail with current rules");
        assert!(parse_expr("...").is_err(), "Triple dot symbol should fail with current rules");
    }
}
