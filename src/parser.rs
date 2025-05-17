
use nom::{
    character::complete::multispace0, // For handling whitespace, removed multispace1
    // combinator::map, // Removed as Parser::map method is used
    number::complete::double,                        // For parsing f64 numbers
    sequence::delimited,                             // For parsers surrounded by other parsers
    IResult,
    Parser, // Import the Parser trait to use its methods like .map() and .parse()
    // Add other nom components as needed, e.g., character, sequence, branch, multi
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

// Top-level parser function for a single expression
// For now, it only tries to parse a number.
// Later, this will use `alt` to try parsing different types of expressions (symbols, lists, etc.)
#[tracing::instrument(level = "trace", skip(input), fields(input = %input))]
pub fn parse_expr(input: &str) -> IResult<&str, Expr> {
    trace!("Attempting to parse expression");
    // For now, we only try to parse a number.
    // In the future, this will be `alt((parse_number, parse_symbol, parse_list, ...))`
    parse_number(input)
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
}
