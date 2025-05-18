pub mod math;
pub mod log;
pub mod special_forms;
    
use crate::engine::ast::Expr;
// LispFunction is not used directly here anymore
use crate::engine::env::Environment; // Still needed for native_module_ref tests if they use Environment::new
use crate::engine::eval::LispError;
// std::cell::RefCell and std::rc::Rc might only be needed for tests now
use std::cell::RefCell; // Keep if tests use it directly
use std::rc::Rc; // Keep if tests use it directly
use tracing::trace; // debug, error, instrument are not used at this level anymore
    
// Future built-in functions will go here.
    
// Native Rust functions callable from Lisp (the "prelude" functions)
// Math functions (native_add, native_equals, native_multiply) are now in the math submodule.
    
#[tracing::instrument(skip(args), ret, err)]
pub fn native_module_ref(args: Vec<Expr>) -> Result<Expr, LispError> {
    trace!("Executing native 'module-ref' function");
    if args.len() != 2 {
        return Err(LispError::ArityMismatch(format!(
            "'module-ref' expects 2 arguments (module-expression, member-symbol), got {}",
            args.len()
        )));
    }

    let module_expr = &args[0];
    let member_symbol_expr = &args[1];

    let lisp_module = match module_expr {
        Expr::Module(m) => m,
        _ => {
            return Err(LispError::TypeError {
                expected: "Module".to_string(),
                found: format!("{:?}", module_expr),
            });
        }
    };

    let member_name = match member_symbol_expr {
        Expr::Symbol(s) => s,
        _ => {
            return Err(LispError::TypeError {
                expected: "Symbol (for member name)".to_string(),
                found: format!("{:?}", member_symbol_expr),
            });
        }
    };

    match lisp_module.env.borrow().get(member_name) {
        Some(value) => Ok(value),
        None => Err(LispError::MemberNotFoundInModule {
            module: lisp_module.path.display().to_string(),
            member: member_name.clone(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{native_module_ref}; // Updated imports
    use crate::engine::ast::{Expr, LispFunction, LispModule, NativeFunction}; // Added NativeFunction & LispModule
    use crate::engine::env::Environment;
    use crate::engine::eval::{LispError, eval}; // Need main eval for testing integration
    use crate::logging::init_test_logging; // Use new logging setup
    use std::path::PathBuf; // Import PathBuf for tests
    use std::rc::Rc; // For Environment

    // Tests for native functions
    // These tests manually add the native functions to the environment.
    // Tests for native_log_info and native_log_error are in builtins/log/mod.rs
    // Tests for native_module_ref
    #[test]
    fn test_native_module_ref_success() {
        init_test_logging();
        let module_env = Environment::new();
        module_env
            .borrow_mut()
            .define("var".to_string(), Expr::Number(123.0));
        let lisp_module = Expr::Module(LispModule {
            path: PathBuf::from("test_module.lisp"),
            env: module_env,
        });
        let member_symbol = Expr::Symbol("var".to_string());

        let args = vec![lisp_module, member_symbol];
        assert_eq!(native_module_ref(args), Ok(Expr::Number(123.0)));
    }

    #[test]
    fn test_native_module_ref_member_not_found() {
        init_test_logging();
        let module_env = Environment::new(); // Empty env
        let lisp_module = Expr::Module(LispModule {
            path: PathBuf::from("test_module.lisp"),
            env: module_env,
        });
        let member_symbol = Expr::Symbol("non_existent_var".to_string());

        let args = vec![lisp_module, member_symbol];
        assert_eq!(
            native_module_ref(args),
            Err(LispError::MemberNotFoundInModule {
                module: "test_module.lisp".to_string(),
                member: "non_existent_var".to_string(),
            })
        );
    }

    #[test]
    fn test_native_module_ref_arity_error_too_few() {
        init_test_logging();
        let lisp_module = Expr::Module(LispModule {
            path: PathBuf::from("test_module.lisp"),
            env: Environment::new(),
        });
        let args = vec![lisp_module]; // Only one arg
        assert_eq!(
            native_module_ref(args),
            Err(LispError::ArityMismatch(
                "'module-ref' expects 2 arguments (module-expression, member-symbol), got 1"
                    .to_string()
            ))
        );
    }

    #[test]
    fn test_native_module_ref_arity_error_too_many() {
        init_test_logging();
        let lisp_module = Expr::Module(LispModule {
            path: PathBuf::from("test_module.lisp"),
            env: Environment::new(),
        });
        let member_symbol = Expr::Symbol("var".to_string());
        let extra_arg = Expr::Nil;
        let args = vec![lisp_module, member_symbol, extra_arg]; // Three args
        assert_eq!(
            native_module_ref(args),
            Err(LispError::ArityMismatch(
                "'module-ref' expects 2 arguments (module-expression, member-symbol), got 3"
                    .to_string()
            ))
        );
    }

    #[test]
    fn test_native_module_ref_first_arg_not_module() {
        init_test_logging();
        let not_a_module = Expr::Number(1.0);
        let member_symbol = Expr::Symbol("var".to_string());
        let args = vec![not_a_module.clone(), member_symbol];
        assert_eq!(
            native_module_ref(args),
            Err(LispError::TypeError {
                expected: "Module".to_string(),
                found: format!("{:?}", not_a_module),
            })
        );
    }

    #[test]
    fn test_native_module_ref_second_arg_not_symbol() {
        init_test_logging();
        let lisp_module = Expr::Module(LispModule {
            path: PathBuf::from("test_module.lisp"),
            env: Environment::new(),
        });
        let not_a_symbol = Expr::Number(123.0);
        let args = vec![lisp_module, not_a_symbol.clone()];
        assert_eq!(
            native_module_ref(args),
            Err(LispError::TypeError {
                expected: "Symbol (for member name)".to_string(),
                found: format!("{:?}", not_a_symbol),
            })
        );
    }
}
