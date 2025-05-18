use crate::engine::ast::Expr;
use crate::engine::eval::LispError;
use tracing::{error, instrument, trace};

#[instrument(skip(args), fields(args = ?args), ret, err)]
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
    Ok(args[0].clone())
}

#[cfg(test)]
mod tests {
    use crate::engine::ast::Expr;
    use crate::engine::env::Environment;
    use crate::engine::eval::{eval, LispError};
    use crate::logging::init_test_logging;
    // Rc is not directly used in these tests. Environment::new() returns Rc<RefCell<Environment>>.

    #[test]
    fn eval_quote_symbol() {
        init_test_logging();
        let env = Environment::new();
        let expr = Expr::List(vec![
            Expr::Symbol("quote".to_string()),
            Expr::Symbol("x".to_string()),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Symbol("x".to_string())));
    }

    #[test]
    fn eval_quote_number() {
        init_test_logging();
        let env = Environment::new();
        let expr = Expr::List(vec![Expr::Symbol("quote".to_string()), Expr::Number(10.0)]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(10.0)));
    }

    #[test]
    fn eval_quote_list() {
        init_test_logging();
        let env = Environment::new();
        let inner_list = vec![Expr::Number(1.0), Expr::Number(2.0)];
        let expr = Expr::List(vec![
            Expr::Symbol("quote".to_string()),
            Expr::List(inner_list.clone()),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::List(inner_list)));
    }

    #[test]
    fn eval_quote_empty_list_as_arg() {
        init_test_logging();
        let env = Environment::new();
        let expr = Expr::List(vec![Expr::Symbol("quote".to_string()), Expr::List(vec![])]);
        assert_eq!(eval(&expr, env), Ok(Expr::List(vec![])));
    }

    #[test]
    fn eval_quote_nested_list() {
        init_test_logging();
        let env = Environment::new();
        let nested_list = Expr::List(vec![
            Expr::Symbol("a".to_string()),
            Expr::List(vec![
                Expr::Symbol("b".to_string()),
                Expr::Symbol("c".to_string()),
            ]),
        ]);
        let expr = Expr::List(vec![Expr::Symbol("quote".to_string()), nested_list.clone()]);
        assert_eq!(eval(&expr, env), Ok(nested_list));
    }

    #[test]
    fn eval_quote_arity_error_no_args() {
        init_test_logging();
        let env = Environment::new();
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
        init_test_logging();
        let env = Environment::new();
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
