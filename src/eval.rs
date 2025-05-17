use crate::ast::Expr;
use thiserror::Error;
use tracing::{debug, error, instrument, trace};

#[derive(Error, Debug, Clone, PartialEq)]
pub enum LispError {
    #[error("Evaluation error: {0}")]
    Evaluation(String),
    #[error("Type error: expected {expected}, found {found}")]
    TypeError { expected: String, found: String },
    #[error("Undefined symbol: {0}")]
    UndefinedSymbol(String),
    #[error("Invalid arguments for operator '{operator}': {message}")]
    InvalidArguments { operator: String, message: String },
    #[error("Arity mismatch: {0}")]
    ArityMismatch(String),
    // Add more specific errors as the interpreter develops
}

#[instrument(skip(expr), fields(expr = ?expr), ret, err)]
pub fn eval(expr: &Expr /*, env: &mut Environment */) -> Result<Expr, LispError> {
    trace!("Starting evaluation");
    // The environment will be needed for variable lookups and definitions.
    // For now, evaluation is very limited.
    match expr {
        Expr::Number(_) => {
            debug!("Evaluating Number: {:?}", expr);
            Ok(expr.clone()) // Numbers evaluate to themselves
        }
        Expr::Symbol(s) => {
            debug!("Evaluating Symbol: {}", s);
            // Symbol evaluation requires an environment to look up its value.
            // For now, all symbols are considered undefined.
            error!(symbol = %s, "Undefined symbol encountered");
            Err(LispError::UndefinedSymbol(s.clone()))
        }
        Expr::List(list) => {
            debug!("Evaluating List: {:?}", list);
            if list.is_empty() {
                trace!("List is empty, evaluating to empty list");
                // An empty list `()` typically evaluates to itself or a 'nil' equivalent.
                Ok(Expr::List(Vec::new()))
            } else {
                trace!("List is not empty, attempting to evaluate as function/special form (not implemented)");
                // This is where function calls and special forms would be handled.
                // For example, if list[0] is a symbol like '+', it would be a function call.
                // This part will be significantly expanded.
                error!("List evaluation (function calls, special forms) not yet implemented for: {:?}", list);
                Err(LispError::Evaluation(
                    "List evaluation (function calls, special forms) not yet implemented".to_string(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*; // Imports eval, Expr, LispError

    // Helper to initialize tracing for tests, ensuring it's only done once.
    // And that test output is captured by the test runner.
    fn setup_tracing() {
        use std::sync::Once;
        static TRACING_INIT: Once = Once::new();
        TRACING_INIT.call_once(|| {
            tracing_subscriber::fmt()
                .with_env_filter("trace") // Show all traces for tests
                .with_test_writer() // Capture output for tests
                .try_init()
                .ok(); // Ignore error if already initialized by another test
        });
    }

    #[test]
    fn eval_number() {
        setup_tracing();
        let expr = Expr::Number(42.0);
        assert_eq!(eval(&expr), Ok(Expr::Number(42.0)));
    }

    #[test]
    fn eval_symbol_undefined() {
        setup_tracing();
        let expr = Expr::Symbol("my_var".to_string());
        assert_eq!(eval(&expr), Err(LispError::UndefinedSymbol("my_var".to_string())));
    }

    #[test]
    fn eval_empty_list() {
        setup_tracing();
        let expr = Expr::List(vec![]);
        assert_eq!(eval(&expr), Ok(Expr::List(vec![])));
    }

    #[test]
    fn eval_non_empty_list_not_implemented() {
        setup_tracing();
        let expr = Expr::List(vec![Expr::Symbol("foo".to_string()), Expr::Number(1.0)]);
        assert_eq!(
            eval(&expr),
            Err(LispError::Evaluation(
                "List evaluation (function calls, special forms) not yet implemented".to_string()
            ))
        );
    }
}
