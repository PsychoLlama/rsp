use crate::engine::ast::{Expr, NativeFunction};
use crate::engine::env::Environment;
use crate::engine::eval::LispError;
use std::collections::HashMap;
use tracing::{error, trace};

fn native_list_length(args: Vec<Expr>) -> Result<Expr, LispError> {
    trace!("Executing native list function: list/length");
    if args.len() != 1 {
        let msg = format!("list/length expects 1 argument, got {}", args.len());
        error!("{}", msg);
        return Err(LispError::ArityMismatch(msg));
    }

    match &args[0] {
        Expr::List(list) => Ok(Expr::Number(list.len() as f64)),
        Expr::Nil => Ok(Expr::Number(0.0)), // An empty list (nil) has length 0
        other => {
            let msg = format!(
                "list/length expects a list or nil as argument, got {:?}",
                other
            );
            error!("{}", msg);
            Err(LispError::TypeError {
                expected: "List or Nil".to_string(),
                found: format!("{:?}", other),
            })
        }
    }
}

/// Creates the `list` module with its associated functions.
pub fn create_list_module() -> Expr {
    trace!("Creating list module");
    let list_env_rc = Environment::new(); // Modules have their own environment

    // Scope the mutable borrow so it's dropped before list_env_rc is moved
    {
        let mut list_env_borrowed = list_env_rc.borrow_mut();
        let functions_to_define: HashMap<String, Expr> = HashMap::from([(
            "length".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "list/length".to_string(), // Convention: module_name/function_name
                func: native_list_length,
            }),
        )]);

        for (name, func_expr) in functions_to_define {
            list_env_borrowed.define(name, func_expr);
        }
    }

    Expr::Module(crate::engine::ast::LispModule {
        // Using a temporary path, or deciding on a convention for "virtual" modules
        path: std::path::PathBuf::from("<builtin_list_module>"),
        env: list_env_rc,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ast::Expr;
    use crate::engine::env::Environment;
    use crate::engine::eval::{eval, LispError};
    use crate::engine::parser::parse_expr;
    use crate::logging::init_test_logging;
    // Removed unused imports for RefCell and Rc for the test module

    // Helper to evaluate a Lisp string in an environment that includes the list module.
    fn eval_list_str(code: &str) -> Result<Expr, LispError> {
        init_test_logging();
        let env = Environment::new_with_prelude(); // Includes math, log, string, list
        let parse_result = parse_expr(code);
        let (remaining, parsed_expr_option) = match parse_result {
            Ok((rem, expr_opt)) => (rem, expr_opt),
            Err(e) => panic!("Test parse error for code '{}': {}", code, e),
        };

        if !remaining.is_empty() {
            panic!(
                "Unexpected remaining input after parsing in test for code '{}': {}",
                code, remaining
            );
        }
        let parsed_expr = parsed_expr_option.expect("Parsed expression should not be None in test");
        eval(&parsed_expr, env)
    }

    #[test]
    fn test_native_list_length_empty_list() {
        let result = eval_list_str("(list/length '())").unwrap();
        assert_eq!(result, Expr::Number(0.0));
    }

    #[test]
    fn test_native_list_length_nil_direct() {
        // Note: `nil` itself is not directly callable as a list function argument
        // in this manner without `quote`. We test `list/length` with `nil` argument.
        // The function `native_list_length` handles `Expr::Nil` directly.
        // The `env` variable was unused here as native_list_length doesn't need it for this direct call.
        init_test_logging(); // Ensure logging is initialized for direct native function tests too
        let result = native_list_length(vec![Expr::Nil]).unwrap();
        assert_eq!(result, Expr::Number(0.0));
    }

    #[test]
    fn test_native_list_length_non_empty_list() {
        let result = eval_list_str("(list/length '(1 2 3))").unwrap();
        assert_eq!(result, Expr::Number(3.0));

        let result_nested = eval_list_str("(list/length '(1 (2 3) 4))").unwrap();
        assert_eq!(result_nested, Expr::Number(3.0));
    }

    #[test]
    fn test_native_list_length_arity_error_no_args() {
        let result = eval_list_str("(list/length)");
        assert!(matches!(result, Err(LispError::ArityMismatch(_))));
    }

    #[test]
    fn test_native_list_length_arity_error_too_many_args() {
        let result = eval_list_str("(list/length '(1) '(2))");
        assert!(matches!(result, Err(LispError::ArityMismatch(_))));
    }

    #[test]
    fn test_native_list_length_type_error_not_a_list() {
        let result = eval_list_str("(list/length 123)");
        assert!(matches!(result, Err(LispError::TypeError { .. })));

        let result_string = eval_list_str("(list/length \"hello\")");
        assert!(matches!(result_string, Err(LispError::TypeError { .. })));
    }
}
