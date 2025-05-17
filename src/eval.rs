use thiserror::Error;
use crate::ast::Expr;

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

pub fn eval(expr: &Expr /*, env: &mut Environment */) -> Result<Expr, LispError> {
    // The environment will be needed for variable lookups and definitions.
    // For now, evaluation is very limited.
    match expr {
        Expr::Number(_) => Ok(expr.clone()), // Numbers evaluate to themselves
        Expr::Symbol(s) => {
            // Symbol evaluation requires an environment to look up its value.
            // For now, all symbols are considered undefined.
            Err(LispError::UndefinedSymbol(s.clone()))
        }
        Expr::List(list) => {
            if list.is_empty() {
                // An empty list `()` typically evaluates to itself or a 'nil' equivalent.
                Ok(Expr::List(Vec::new()))
            } else {
                // This is where function calls and special forms would be handled.
                // For example, if list[0] is a symbol like '+', it would be a function call.
                // This part will be significantly expanded.
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

    #[test]
    fn eval_number() {
        let expr = Expr::Number(42.0);
        assert_eq!(eval(&expr), Ok(Expr::Number(42.0)));
    }

    #[test]
    fn eval_symbol_undefined() {
        let expr = Expr::Symbol("my_var".to_string());
        assert_eq!(eval(&expr), Err(LispError::UndefinedSymbol("my_var".to_string())));
    }

    #[test]
    fn eval_empty_list() {
        let expr = Expr::List(vec![]);
        assert_eq!(eval(&expr), Ok(Expr::List(vec![])));
    }

    #[test]
    fn eval_non_empty_list_not_implemented() {
        let expr = Expr::List(vec![Expr::Symbol("foo".to_string()), Expr::Number(1.0)]);
        assert_eq!(
            eval(&expr),
            Err(LispError::Evaluation(
                "List evaluation (function calls, special forms) not yet implemented".to_string()
            ))
        );
    }
}
