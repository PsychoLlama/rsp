use crate::ast::{Expr, LispFunction};
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

    // Check if the variable name is a reserved keyword
    if crate::special_forms::is_special_form(&var_name) {
        error!(attempted_keyword = %var_name, "Attempted to bind a reserved keyword using 'let'");
        return Err(LispError::ReservedKeyword(var_name));
    }

    debug!(variable_name = %var_name, value_expression = ?value_expr, "'let' binding");
    // Note: We need to call back into the main eval function here.
    // This requires `crate::eval::eval` to be accessible.
    let evaluated_value = crate::eval::eval(value_expr, Rc::clone(&env))?;

    env.borrow_mut()
        .define(var_name.clone(), evaluated_value.clone());
    debug!(variable_name = %var_name, value = ?evaluated_value, "Defined variable in environment using 'let'");
    Ok(evaluated_value)
}

#[tracing::instrument(skip(args, env), fields(args = ?args), ret, err)]
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
    let body_expr = args[1].clone(); // Clone body to take ownership

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
                // Check if parameter name is a reserved keyword
                if crate::special_forms::is_special_form(name) {
                    error!(attempted_keyword = %name, "Attempted to use a reserved keyword as a function parameter");
                    return Err(LispError::ReservedKeyword(name.clone()));
                }
                param_names.push(name.clone());
            }
            _ => {
                error!(
                    "Parameters in 'fn' must be symbols, found {:?}",
                    param
                );
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
        closure: Rc::clone(&env), // Capture the current environment
    };

    Ok(Expr::Function(lisp_fn))
}

#[tracing::instrument(skip(args), fields(args = ?args), ret, err)]
pub fn eval_quote(args: &[Expr]) -> Result<Expr, LispError> {
    trace!("Executing 'quote' special form");
    if args.len() != 1 {
        error!(
            "'quote' special form requires 1 argument, found {}",
            args.len()
        );
        return Err(LispError::ArityMismatch(format!(
            "'quote' expects 1 argument, got {}",
            args.len()
        )));
    }
    // The argument to quote is not evaluated.
    Ok(args[0].clone())
}

#[tracing::instrument(skip(args, env), fields(args = ?args), ret, err)]
pub fn eval_if(args: &[Expr], env: Rc<RefCell<Environment>>) -> Result<Expr, LispError> {
    trace!("Executing 'if' special form");
    if args.len() < 2 || args.len() > 3 {
        error!(
            "'if' special form requires 2 or 3 arguments (condition, then-branch, [else-branch]), found {}",
            args.len()
        );
        return Err(LispError::ArityMismatch(format!(
            "'if' expects 2 or 3 arguments, got {}",
            args.len()
        )));
    }

    let condition_expr = &args[0];
    let then_expr = &args[1];
    let else_expr_opt = args.get(2);

    let condition_result = crate::eval::eval(condition_expr, Rc::clone(&env))?;
    debug!(?condition_result, "Evaluated 'if' condition");

    match condition_result {
        Expr::Bool(false) | Expr::Nil => {
            // Condition is false or nil, evaluate else-branch or return Nil
            if let Some(else_expr) = else_expr_opt {
                trace!("Condition is false-y, evaluating else-branch");
                crate::eval::eval(else_expr, env)
            } else {
                trace!("Condition is false-y, no else-branch, returning Nil");
                Ok(Expr::Nil)
            }
        }
        _ => {
            // Condition is truthy (anything not false or Nil)
            trace!("Condition is truthy, evaluating then-branch");
            crate::eval::eval(then_expr, env)
        }
    }
}

// Future built-in functions will go here.

#[cfg(test)]
mod tests {
    use crate::ast::{Expr, LispFunction};
    use crate::env::Environment;
    use crate::eval::{LispError, eval}; // Need main eval for testing integration
    use crate::test_utils::setup_tracing; // Use shared setup_tracing
    use std::rc::Rc; // For Environment

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

    #[test]
    fn eval_let_error_binding_reserved_keyword_let() {
        setup_tracing();
        let env = Environment::new();
        // (let let 10)
        let expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Symbol("let".to_string()), // Variable name is "let"
            Expr::Number(10.0),
        ]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::ReservedKeyword("let".to_string()))
        );
    }

    #[test]
    fn eval_let_error_binding_reserved_keyword_quote() {
        setup_tracing();
        let env = Environment::new();
        // (let quote 10)
        let expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Symbol("quote".to_string()), // Variable name is "quote"
            Expr::Number(10.0),
        ]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::ReservedKeyword("quote".to_string()))
        );
    }

    // Tests for eval_fn
    #[test]
    fn eval_fn_creates_function() {
        setup_tracing();
        let env = Environment::new();
        // (fn (x y) x)
        let fn_expr_ast = Expr::List(vec![
            Expr::Symbol("fn".to_string()),
            Expr::List(vec![
                Expr::Symbol("x".to_string()),
                Expr::Symbol("y".to_string()),
            ]),
            Expr::Symbol("x".to_string()),
        ]);

        // We call eval here because 'fn' is a special form handled by eval, which then calls eval_fn
        let result = eval(&fn_expr_ast, Rc::clone(&env));

        match result {
            Ok(Expr::Function(LispFunction { params, body, closure })) => {
                assert_eq!(params, vec!["x".to_string(), "y".to_string()]);
                assert_eq!(*body, Expr::Symbol("x".to_string()));
                // Check if the closure is the environment we passed.
                // This is a bit tricky to assert directly beyond pointer equality if available,
                // or by side-effect (e.g. defining something in `env` and checking if `closure` sees it).
                // For now, we trust it's cloned. Rc::ptr_eq(&closure, &env) would be a direct check.
                assert!(Rc::ptr_eq(&closure, &env));
            }
            _ => panic!("Expected LispFunction, got {:?}", result),
        }
    }

    #[test]
    fn eval_fn_empty_params() {
        setup_tracing();
        let env = Environment::new();
        // (fn () 10)
        let fn_expr_ast = Expr::List(vec![
            Expr::Symbol("fn".to_string()),
            Expr::List(vec![]), // Empty parameter list
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
        setup_tracing();
        let env = Environment::new();
        // (fn (x)) - missing body
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
        setup_tracing();
        let env = Environment::new();
        // (fn (x) x x) - extra argument
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
        setup_tracing();
        let env = Environment::new();
        // (fn x x) - first arg (params) is not a list
        let fn_expr_ast = Expr::List(vec![
            Expr::Symbol("fn".to_string()),
            Expr::Symbol("x".to_string()), // Not a list
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
        setup_tracing();
        let env = Environment::new();
        // (fn (x 10) x) - param list contains a number
        let fn_expr_ast = Expr::List(vec![
            Expr::Symbol("fn".to_string()),
            Expr::List(vec![Expr::Symbol("x".to_string()), Expr::Number(10.0)]), // 10.0 is not a symbol
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
        setup_tracing();
        let env = Environment::new();
        // (fn (let) let)
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

    // Tests for 'quote' special form (invoked via eval)
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

    // Tests for 'if' special form
    #[test]
    fn eval_if_true_condition() {
        setup_tracing();
        let env = Environment::new();
        // (if true 10 20)
        let expr = Expr::List(vec![
            Expr::Symbol("if".to_string()),
            Expr::Bool(true),
            Expr::Number(10.0),
            Expr::Number(20.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(10.0)));
    }

    #[test]
    fn eval_if_false_condition() {
        setup_tracing();
        let env = Environment::new();
        // (if false 10 20)
        let expr = Expr::List(vec![
            Expr::Symbol("if".to_string()),
            Expr::Bool(false),
            Expr::Number(10.0),
            Expr::Number(20.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(20.0)));
    }

    #[test]
    fn eval_if_nil_condition() {
        setup_tracing();
        let env = Environment::new();
        // (if nil 10 20)
        let expr = Expr::List(vec![
            Expr::Symbol("if".to_string()),
            Expr::Nil,
            Expr::Number(10.0),
            Expr::Number(20.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(20.0)));
    }

    #[test]
    fn eval_if_truthy_number_condition() {
        setup_tracing();
        let env = Environment::new();
        // (if 0 10 20) ; 0 is truthy in this Lisp
        let expr = Expr::List(vec![
            Expr::Symbol("if".to_string()),
            Expr::Number(0.0),
            Expr::Number(10.0),
            Expr::Number(20.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(10.0)));
    }

    #[test]
    fn eval_if_truthy_list_condition() {
        setup_tracing();
        let env = Environment::new();
        // (if () 10 20) ; empty list is truthy
        let expr = Expr::List(vec![
            Expr::Symbol("if".to_string()),
            Expr::List(vec![]),
            Expr::Number(10.0),
            Expr::Number(20.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(10.0)));
    }
    
    #[test]
    fn eval_if_false_condition_no_else_branch() {
        setup_tracing();
        let env = Environment::new();
        // (if false 10)
        let expr = Expr::List(vec![
            Expr::Symbol("if".to_string()),
            Expr::Bool(false),
            Expr::Number(10.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Nil));
    }

    #[test]
    fn eval_if_true_condition_no_else_branch() {
        setup_tracing();
        let env = Environment::new();
        // (if true 10)
        let expr = Expr::List(vec![
            Expr::Symbol("if".to_string()),
            Expr::Bool(true),
            Expr::Number(10.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(10.0)));
    }

    #[test]
    fn eval_if_condition_evaluates() {
        setup_tracing();
        let env = Environment::new();
        env.borrow_mut().define("cond-var".to_string(), Expr::Bool(true));
        // (if cond-var 10 20)
        let expr = Expr::List(vec![
            Expr::Symbol("if".to_string()),
            Expr::Symbol("cond-var".to_string()),
            Expr::Number(10.0),
            Expr::Number(20.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(10.0)));
    }

    #[test]
    fn eval_if_arity_error_too_few_args() {
        setup_tracing();
        let env = Environment::new();
        // (if true)
        let expr = Expr::List(vec![
            Expr::Symbol("if".to_string()),
            Expr::Bool(true),
        ]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::ArityMismatch(
                "'if' expects 2 or 3 arguments, got 1".to_string()
            ))
        );
    }

    #[test]
    fn eval_if_arity_error_too_many_args() {
        setup_tracing();
        let env = Environment::new();
        // (if true 10 20 30)
        let expr = Expr::List(vec![
            Expr::Symbol("if".to_string()),
            Expr::Bool(true),
            Expr::Number(10.0),
            Expr::Number(20.0),
            Expr::Number(30.0),
        ]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::ArityMismatch(
                "'if' expects 2 or 3 arguments, got 4".to_string()
            ))
        );
    }

    // Test that only the correct branch is evaluated (short-circuiting)
    // This test defines 'then-val' but not 'else-val'. If 'else-val' were evaluated, it would error.
    #[test]
    fn eval_if_short_circuit_then_branch() {
        setup_tracing();
        let env = Environment::new();
        env.borrow_mut().define("then-val".to_string(), Expr::Number(100.0));
        // (if true then-val else-val) ; else-val is undefined
        let expr = Expr::List(vec![
            Expr::Symbol("if".to_string()),
            Expr::Bool(true),
            Expr::Symbol("then-val".to_string()),
            Expr::Symbol("else-val".to_string()), // This should not be evaluated
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(100.0)));
    }

    // This test defines 'else-val' but not 'then-val'. If 'then-val' were evaluated, it would error.
    #[test]
    fn eval_if_short_circuit_else_branch() {
        setup_tracing();
        let env = Environment::new();
        env.borrow_mut().define("else-val".to_string(), Expr::Number(200.0));
        // (if false then-val else-val) ; then-val is undefined
        let expr = Expr::List(vec![
            Expr::Symbol("if".to_string()),
            Expr::Bool(false),
            Expr::Symbol("then-val".to_string()), // This should not be evaluated
            Expr::Symbol("else-val".to_string()),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(200.0)));
    }
}
