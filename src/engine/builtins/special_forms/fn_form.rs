use crate::engine::ast::{Expr, LispFunction};
use crate::engine::env::Environment;
use crate::engine::eval::LispError;
use crate::engine::special_forms as special_form_constants;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::{debug, error, instrument, trace};

#[instrument(skip(args, env), fields(args = ?args), ret, err)]
pub fn eval_fn(args: &[Expr], env: Rc<RefCell<Environment>>) -> Result<Expr, LispError> {
    trace!("Executing 'fn' special form");
    if args.len() != 2 {
        error!(
            "'fn' special form requires 2 arguments (parameters list and body), found {}",
            args.len()
        );
        return Err(LispError::ArityMismatch(format!(
            "'fn' expects 2 arguments (parameters list and body), got {}",
            args.len()
        )));
    }

    let params_expr = &args[0];
    let body_expr = args[1].clone();

    let params_list = match params_expr {
        Expr::List(list) => list,
        _ => {
            error!(
                "First argument to 'fn' must be a list of parameters, found {:?}",
                params_expr
            );
            return Err(LispError::TypeError {
                expected: "List of parameters".to_string(),
                found: format!("{:?}", params_expr),
            });
        }
    };

    let mut param_names = Vec::new();
    for param in params_list {
        match param {
            Expr::Symbol(name) => {
                if special_form_constants::is_special_form(name) {
                    error!(attempted_keyword = %name, "Attempted to use a reserved keyword as a function parameter");
                    return Err(LispError::ReservedKeyword(name.clone()));
                }
                param_names.push(name.clone());
            }
            _ => {
                error!("Parameters in 'fn' must be symbols, found {:?}", param);
                return Err(LispError::TypeError {
                    expected: "Symbol".to_string(),
                    found: format!("{:?}", param),
                });
            }
        }
    }

    debug!(parameters = ?param_names, body = ?body_expr, "'fn' creating function");
    let lisp_fn = LispFunction {
        params: param_names,
        body: Box::new(body_expr),
        closure: Rc::clone(&env),
    };

    Ok(Expr::Function(lisp_fn))
}

#[cfg(test)]
mod tests {
    use super::eval_fn;
    use crate::engine::ast::{Expr, LispFunction};
    use crate::engine::env::Environment;
    use crate::engine::eval::{eval, LispError};
    use crate::logging::init_test_logging;
    use std::rc::Rc;

    #[test]
    fn eval_fn_creates_function() {
        init_test_logging();
        let env = Environment::new();
        let fn_expr_ast = Expr::List(vec![
            Expr::Symbol("fn".to_string()),
            Expr::List(vec![
                Expr::Symbol("x".to_string()),
                Expr::Symbol("y".to_string()),
            ]),
            Expr::Symbol("x".to_string()),
        ]);

        let result = eval(&fn_expr_ast, Rc::clone(&env));

        match result {
            Ok(Expr::Function(LispFunction {
                params,
                body,
                closure,
            })) => {
                assert_eq!(params, vec!["x".to_string(), "y".to_string()]);
                assert_eq!(*body, Expr::Symbol("x".to_string()));
                assert!(Rc::ptr_eq(&closure, &env));
            }
            _ => panic!("Expected LispFunction, got {:?}", result),
        }
    }

    #[test]
    fn eval_fn_empty_params() {
        init_test_logging();
        let env = Environment::new();
        let fn_expr_ast = Expr::List(vec![
            Expr::Symbol("fn".to_string()),
            Expr::List(vec![]), 
            Expr::Number(10.0),
        ]);
        let result = eval(&fn_expr_ast, Rc::clone(&env));
        match result {
            Ok(Expr::Function(LispFunction { params, body, .. })) => {
                assert_eq!(params, Vec::<String>::new());
                assert_eq!(*body, Expr::Number(10.0));
            }
            _ => panic!("Expected LispFunction, got {:?}", result),
        }
    }

    #[test]
    fn eval_fn_arity_error_too_few_args() {
        init_test_logging();
        let env = Environment::new();
        let fn_expr_ast = Expr::List(vec![
            Expr::Symbol("fn".to_string()),
            Expr::List(vec![Expr::Symbol("x".to_string())]),
        ]);
        assert_eq!(
            eval(&fn_expr_ast, env),
            Err(LispError::ArityMismatch(
                "'fn' expects 2 arguments (parameters list and body), got 1".to_string()
            ))
        );
    }

    #[test]
    fn eval_fn_arity_error_too_many_args() {
        init_test_logging();
        let env = Environment::new();
        let fn_expr_ast = Expr::List(vec![
            Expr::Symbol("fn".to_string()),
            Expr::List(vec![Expr::Symbol("x".to_string())]),
            Expr::Symbol("x".to_string()),
            Expr::Symbol("x".to_string()),
        ]);
        assert_eq!(
            eval(&fn_expr_ast, env),
            Err(LispError::ArityMismatch(
                "'fn' expects 2 arguments (parameters list and body), got 3".to_string()
            ))
        );
    }

    #[test]
    fn eval_fn_param_not_a_list() {
        init_test_logging();
        let env = Environment::new();
        let fn_expr_ast = Expr::List(vec![
            Expr::Symbol("fn".to_string()),
            Expr::Symbol("x".to_string()), 
            Expr::Symbol("x".to_string()),
        ]);
        assert_eq!(
            eval(&fn_expr_ast, env),
            Err(LispError::TypeError {
                expected: "List of parameters".to_string(),
                found: "Symbol(\"x\")".to_string()
            })
        );
    }

    #[test]
    fn eval_fn_param_list_contains_non_symbol() {
        init_test_logging();
        let env = Environment::new();
        let fn_expr_ast = Expr::List(vec![
            Expr::Symbol("fn".to_string()),
            Expr::List(vec![Expr::Symbol("x".to_string()), Expr::Number(10.0)]), 
            Expr::Symbol("x".to_string()),
        ]);
        assert_eq!(
            eval(&fn_expr_ast, env),
            Err(LispError::TypeError {
                expected: "Symbol".to_string(),
                found: "Number(10.0)".to_string()
            })
        );
    }

    #[test]
    fn eval_fn_param_is_reserved_keyword() {
        init_test_logging();
        let env = Environment::new();
        let fn_expr_ast = Expr::List(vec![
            Expr::Symbol("fn".to_string()),
            Expr::List(vec![Expr::Symbol("let".to_string())]),
            Expr::Symbol("let".to_string()),
        ]);
        assert_eq!(
            eval(&fn_expr_ast, env),
            Err(LispError::ReservedKeyword("let".to_string()))
        );
    }
}
