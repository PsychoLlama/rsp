use crate::ast::Expr;
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
                    trace!(symbol_name = %s, "Handling 'let' special form");
                    if list.len() != 3 {
                        error!(
                            "'let' special form requires 2 arguments (variable name and value), found {}",
                            list.len() - 1
                        );
                        return Err(LispError::ArityMismatch(format!(
                            "'let' expects 2 arguments, got {}",
                            list.len() - 1
                        )));
                    }

                    let var_name_expr = &list[1];
                    let value_expr = &list[2];

                    let var_name = match var_name_expr {
                        Expr::Symbol(name) => name.clone(),
                        _ => {
                            error!(
                                "First argument to 'let' must be a symbol, found {:?}",
                                var_name_expr
                            );
                            return Err(LispError::TypeError {
                                expected: "Symbol".to_string(),
                                found: format!("{:?}", var_name_expr),
                            });
                        }
                    };

                    debug!(variable_name = %var_name, value_expression = ?value_expr, "'let' binding");
                    let evaluated_value = eval(value_expr, Rc::clone(&env))?;

                    env.borrow_mut()
                        .define(var_name.clone(), evaluated_value.clone());
                    debug!(variable_name = %var_name, value = ?evaluated_value, "Defined variable in environment using 'let'");
                    Ok(evaluated_value)
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

    // Helper to initialize tracing for tests, ensuring it's only done once.
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

    #[test]
    fn eval_let_binding() {
        setup_tracing();
        let env = Environment::new();
        // (let x 10)
        let let_expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Symbol("x".to_string()),
            Expr::Number(10.0),
        ]);
        // `let` should evaluate to the bound value
        assert_eq!(eval(&let_expr, Rc::clone(&env)), Ok(Expr::Number(10.0)));
        // Check if 'x' is defined in the environment
        assert_eq!(env.borrow().get("x"), Some(Expr::Number(10.0)));

        // Evaluate 'x' after binding
        let x_sym = Expr::Symbol("x".to_string());
        assert_eq!(eval(&x_sym, Rc::clone(&env)), Ok(Expr::Number(10.0)));
    }

    #[test]
    fn eval_let_binding_evaluates_value() {
        setup_tracing();
        let env = Environment::new();
        env.borrow_mut().define("y".to_string(), Expr::Number(5.0));
        // (let x (some_expr_evaluating_to_y_times_2)) - for now, just use y
        // (let x y) where y is 5
        let let_expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Symbol("x".to_string()),
            Expr::Symbol("y".to_string()), // This will be evaluated
        ]);
        assert_eq!(eval(&let_expr, Rc::clone(&env)), Ok(Expr::Number(5.0)));
        assert_eq!(env.borrow().get("x"), Some(Expr::Number(5.0)));
    }

    #[test]
    fn eval_let_arity_error_too_few_args() {
        setup_tracing();
        let env = Environment::new();
        // (let x) - missing value
        let let_expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Symbol("x".to_string()),
        ]);
        assert_eq!(
            eval(&let_expr, env),
            Err(LispError::ArityMismatch(
                "'let' expects 2 arguments, got 1".to_string()
            ))
        );
    }

    #[test]
    fn eval_let_arity_error_too_many_args() {
        setup_tracing();
        let env = Environment::new();
        // (let x 10 20) - extra argument
        let let_expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Symbol("x".to_string()),
            Expr::Number(10.0),
            Expr::Number(20.0),
        ]);
        assert_eq!(
            eval(&let_expr, env),
            Err(LispError::ArityMismatch(
                "'let' expects 2 arguments, got 3".to_string()
            ))
        );
    }

    #[test]
    fn eval_let_type_error_non_symbol_for_var_name() {
        setup_tracing();
        let env = Environment::new();
        // (let 10 20) - first arg (var name) is not a symbol
        let let_expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Number(10.0), // Not a symbol
            Expr::Number(20.0),
        ]);
        assert_eq!(
            eval(&let_expr, env),
            Err(LispError::TypeError {
                expected: "Symbol".to_string(),
                found: "Number(10.0)".to_string()
            })
        );
    }
}
