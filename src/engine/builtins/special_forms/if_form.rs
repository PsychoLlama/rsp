use crate::engine::ast::Expr;
use crate::engine::env::Environment;
use crate::engine::eval::{eval as main_eval, LispError};
use std::cell::RefCell;
use std::rc::Rc;
use tracing::{debug, error, instrument, trace};

#[instrument(skip(args, env), fields(args = ?args), ret, err)]
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

    let condition_result = main_eval(condition_expr, Rc::clone(&env))?;
    debug!(?condition_result, "Evaluated 'if' condition");

    match condition_result {
        Expr::Bool(false) | Expr::Nil => {
            if let Some(else_expr) = else_expr_opt {
                trace!("Condition is false-y, evaluating else-branch");
                main_eval(else_expr, env)
            } else {
                trace!("Condition is false-y, no else-branch, returning Nil");
                Ok(Expr::Nil)
            }
        }
        _ => {
            trace!("Condition is truthy, evaluating then-branch");
            main_eval(then_expr, env)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::eval_if;
    use crate::engine::ast::Expr;
    use crate::engine::env::Environment;
    use crate::engine::eval::{eval, LispError};
    use crate::logging::init_test_logging;
    use std::rc::Rc;

    #[test]
    fn eval_if_true_condition() {
        init_test_logging();
        let env = Environment::new();
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
        init_test_logging();
        let env = Environment::new();
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
        init_test_logging();
        let env = Environment::new();
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
        init_test_logging();
        let env = Environment::new();
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
        init_test_logging();
        let env = Environment::new();
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
        init_test_logging();
        let env = Environment::new();
        let expr = Expr::List(vec![
            Expr::Symbol("if".to_string()),
            Expr::Bool(false),
            Expr::Number(10.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Nil));
    }

    #[test]
    fn eval_if_true_condition_no_else_branch() {
        init_test_logging();
        let env = Environment::new();
        let expr = Expr::List(vec![
            Expr::Symbol("if".to_string()),
            Expr::Bool(true),
            Expr::Number(10.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(10.0)));
    }

    #[test]
    fn eval_if_condition_evaluates() {
        init_test_logging();
        let env = Environment::new();
        env.borrow_mut()
            .define("cond-var".to_string(), Expr::Bool(true));
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
        init_test_logging();
        let env = Environment::new();
        let expr = Expr::List(vec![Expr::Symbol("if".to_string()), Expr::Bool(true)]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::ArityMismatch(
                "'if' expects 2 or 3 arguments, got 1".to_string()
            ))
        );
    }

    #[test]
    fn eval_if_arity_error_too_many_args() {
        init_test_logging();
        let env = Environment::new();
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

    #[test]
    fn eval_if_short_circuit_then_branch() {
        init_test_logging();
        let env = Environment::new();
        env.borrow_mut()
            .define("then-val".to_string(), Expr::Number(100.0));
        let expr = Expr::List(vec![
            Expr::Symbol("if".to_string()),
            Expr::Bool(true),
            Expr::Symbol("then-val".to_string()),
            Expr::Symbol("else-val".to_string()), 
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(100.0)));
    }

    #[test]
    fn eval_if_short_circuit_else_branch() {
        init_test_logging();
        let env = Environment::new();
        env.borrow_mut()
            .define("else-val".to_string(), Expr::Number(200.0));
        let expr = Expr::List(vec![
            Expr::Symbol("if".to_string()),
            Expr::Bool(false),
            Expr::Symbol("then-val".to_string()), 
            Expr::Symbol("else-val".to_string()),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(200.0)));
    }
}
