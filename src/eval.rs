use crate::ast::Expr;
use crate::builtins; // Added
use crate::env::Environment;
use std::cell::RefCell;
use std::rc::Rc;
use thiserror::Error;
use tracing::{debug, error, instrument, trace, warn};

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

#[instrument(skip(expr, env), fields(expr = ?expr), ret, err)]
pub fn eval(expr: &Expr, env: Rc<RefCell<Environment>>) -> Result<Expr, LispError> {
    trace!("Starting evaluation");
    match expr {
        Expr::Number(_) => {
            debug!(env = ?env.borrow(), "Evaluating Number: {:?}", expr);
            Ok(expr.clone()) // Numbers evaluate to themselves
        }
        Expr::Symbol(s) => {
            debug!(env = ?env.borrow(), symbol_name = %s, "Evaluating Symbol");
            if let Some(value) = env.borrow().get(s) {
                trace!(symbol_name = %s, value = ?value, "Found symbol in environment");
                Ok(value)
            } else {
                error!(symbol_name = %s, "Undefined symbol encountered");
                Err(LispError::UndefinedSymbol(s.clone()))
            }
        }
        Expr::List(list) => {
            debug!(env = ?env.borrow(), "Evaluating List: {:?}", list);
            if list.is_empty() {
                trace!("List is empty, evaluating to empty list");
                return Ok(Expr::List(Vec::new())); // Empty list evaluates to itself
            }

            // Handle special forms and function calls
            let first_form = &list[0];
            match first_form {
                Expr::Symbol(s) if s == "let" => {
                    // Pass arguments *after* 'let' to the handler
                    builtins::eval_let(&list[1..], Rc::clone(&env))
                }
                Expr::Symbol(s) if s == "quote" => {
                    trace!("Executing 'quote' special form");
                    if list.len() != 2 {
                        error!(
                            "'quote' special form requires 1 argument, found {}",
                            list.len() - 1
                        );
                        return Err(LispError::ArityMismatch(format!(
                            "'quote' expects 1 argument, got {}",
                            list.len() - 1
                        )));
                    }
                    // The argument to quote is not evaluated.
                    Ok(list[1].clone())
                }
                // Placeholder for other function calls or special forms
                _ => {
                    trace!(
                        "List is not empty, and first element is not a recognized special form. Attempting to evaluate as function/special form (not implemented)"
                    );
                    // This is where other function calls and special forms would be handled.
                    warn!(
                        ?list,
                        "List evaluation (function calls, special forms) not yet implemented for this case"
                    );
                    Err(LispError::Evaluation(format!(
                        "Don't know how to evaluate list starting with: {:?}",
                        first_form
                    )))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*; // Imports eval, Expr, LispError, Environment, Rc, RefCell
    use crate::test_utils::setup_tracing; // Use shared setup_tracing

    #[test]
    fn eval_number() {
        setup_tracing();
        let env = Environment::new();
        let expr = Expr::Number(42.0);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(42.0)));
    }

    #[test]
    fn eval_symbol_defined_in_env() {
        setup_tracing();
        let env = Environment::new();
        env.borrow_mut()
            .define("x".to_string(), Expr::Number(100.0));
        let expr = Expr::Symbol("x".to_string());
        assert_eq!(eval(&expr, env), Ok(Expr::Number(100.0)));
    }

    #[test]
    fn eval_symbol_defined_in_outer_env() {
        setup_tracing();
        let outer_env = Environment::new();
        outer_env
            .borrow_mut()
            .define("x".to_string(), Expr::Number(100.0));
        let inner_env = Environment::new_enclosed(outer_env);
        let expr = Expr::Symbol("x".to_string());
        assert_eq!(eval(&expr, inner_env), Ok(Expr::Number(100.0)));
    }

    #[test]
    fn eval_symbol_shadowed() {
        setup_tracing();
        let outer_env = Environment::new();
        outer_env
            .borrow_mut()
            .define("x".to_string(), Expr::Number(100.0));
        let inner_env = Environment::new_enclosed(outer_env.clone());
        inner_env
            .borrow_mut()
            .define("x".to_string(), Expr::Number(200.0)); // Shadow

        let expr = Expr::Symbol("x".to_string());
        assert_eq!(eval(&expr, inner_env), Ok(Expr::Number(200.0)));
        // Ensure outer is not affected by eval call on inner
        assert_eq!(outer_env.borrow().get("x"), Some(Expr::Number(100.0)));
    }

    #[test]
    fn eval_symbol_undefined() {
        setup_tracing();
        let env = Environment::new();
        let expr = Expr::Symbol("my_var".to_string());
        assert_eq!(
            eval(&expr, env),
            Err(LispError::UndefinedSymbol("my_var".to_string()))
        );
    }

    #[test]
    fn eval_empty_list() {
        setup_tracing();
        let env = Environment::new();
        let expr = Expr::List(vec![]);
        assert_eq!(eval(&expr, env), Ok(Expr::List(vec![])));
    }

    #[test]
    fn eval_non_empty_list_not_implemented() {
        setup_tracing();
        let env = Environment::new();
        let expr = Expr::List(vec![
            Expr::Symbol("unknown_function".to_string()),
            Expr::Number(1.0),
        ]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::Evaluation(
                "Don't know how to evaluate list starting with: Symbol(\"unknown_function\")"
                    .to_string()
            ))
        );
    }

    // Tests for 'quote' special form
    #[test]
    fn eval_quote_symbol() {
        setup_tracing();
        let env = Environment::new();
        // (quote x)
        let expr = Expr::List(vec![
            Expr::Symbol("quote".to_string()),
            Expr::Symbol("x".to_string()),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Symbol("x".to_string())));
    }

    #[test]
    fn eval_quote_number() {
        setup_tracing();
        let env = Environment::new();
        // (quote 10)
        let expr = Expr::List(vec![
            Expr::Symbol("quote".to_string()),
            Expr::Number(10.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(10.0)));
    }

    #[test]
    fn eval_quote_list() {
        setup_tracing();
        let env = Environment::new();
        // (quote (1 2))
        let inner_list = vec![Expr::Number(1.0), Expr::Number(2.0)];
        let expr = Expr::List(vec![
            Expr::Symbol("quote".to_string()),
            Expr::List(inner_list.clone()),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::List(inner_list)));
    }

    #[test]
    fn eval_quote_empty_list_as_arg() {
        setup_tracing();
        let env = Environment::new();
        // (quote ())
        let expr = Expr::List(vec![
            Expr::Symbol("quote".to_string()),
            Expr::List(vec![]),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::List(vec![])));
    }

    #[test]
    fn eval_quote_nested_list() {
        setup_tracing();
        let env = Environment::new();
        // (quote (a (b c)))
        let nested_list = Expr::List(vec![
            Expr::Symbol("a".to_string()),
            Expr::List(vec![
                Expr::Symbol("b".to_string()),
                Expr::Symbol("c".to_string()),
            ]),
        ]);
        let expr = Expr::List(vec![
            Expr::Symbol("quote".to_string()),
            nested_list.clone(),
        ]);
        assert_eq!(eval(&expr, env), Ok(nested_list));
    }

    #[test]
    fn eval_quote_arity_error_no_args() {
        setup_tracing();
        let env = Environment::new();
        // (quote)
        let expr = Expr::List(vec![Expr::Symbol("quote".to_string())]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::ArityMismatch(
                "'quote' expects 1 argument, got 0".to_string()
            ))
        );
    }

    #[test]
    fn eval_quote_arity_error_too_many_args() {
        setup_tracing();
        let env = Environment::new();
        // (quote x y)
        let expr = Expr::List(vec![
            Expr::Symbol("quote".to_string()),
            Expr::Symbol("x".to_string()),
            Expr::Symbol("y".to_string()),
        ]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::ArityMismatch(
                "'quote' expects 1 argument, got 2".to_string()
            ))
        );
    }
}
