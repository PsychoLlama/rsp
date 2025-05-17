use crate::ast::Expr;
use crate::env::Environment;
use crate::eval::LispError; // eval_let needs to return LispError
use std::cell::RefCell;
use std::rc::Rc;
use tracing::{debug, error, trace};

#[tracing::instrument(skip(args, env), fields(args = ?args), ret, err)]
pub fn eval_let(args: &[Expr], env: Rc<RefCell<Environment>>) -> Result<Expr, LispError> {
    trace!("Executing 'let' special form");
    if args.len() != 2 {
        error!(
            "'let' special form requires 2 arguments (variable name and value), found {}",
            args.len()
        );
        return Err(LispError::ArityMismatch(format!(
            "'let' expects 2 arguments, got {}",
            args.len()
        )));
    }

    let var_name_expr = &args[0];
    let value_expr = &args[1];

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
    // Note: We need to call back into the main eval function here.
    // This requires `crate::eval::eval` to be accessible.
    let evaluated_value = crate::eval::eval(value_expr, Rc::clone(&env))?;

    env.borrow_mut()
        .define(var_name.clone(), evaluated_value.clone());
    debug!(variable_name = %var_name, value = ?evaluated_value, "Defined variable in environment using 'let'");
    Ok(evaluated_value)
}

// Future built-in functions will go here.

#[cfg(test)]
mod tests {
    use super::*; // Imports eval_let
    use crate::ast::Expr;
    use crate::env::Environment;
    use crate::eval::{eval, LispError}; // Need main eval for testing integration
    use std::rc::Rc; // For Environment

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
    fn eval_let_binding() {
        setup_tracing();
        let env = Environment::new();
        // (let x 10)
        let let_expr = Expr::List(vec![
            Expr::Symbol("let".to_string()), // This will be handled by eval, dispatching to eval_let
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
        // (let x y) where y is 5
        let let_expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Symbol("x".to_string()),
            Expr::Symbol("y".to_string()), // This will be evaluated by the inner call to `eval`
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
