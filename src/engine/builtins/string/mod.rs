use crate::engine::ast::{Expr, LispModule, NativeFunction};
use crate::engine::env::Environment;
use crate::engine::eval::LispError;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use tracing::{error, trace};

// Helper function to extract a String from an Expr, consistent with extract_number
fn extract_string(expr: &Expr, op_name: &str) -> Result<String, LispError> {
    match expr {
        Expr::String(s) => Ok(s.clone()),
        _ => {
            let type_error = LispError::TypeError {
                expected: "String".to_string(),
                found: format!("{:?}", expr), // Consistent with math/mod.rs
            };
            error!(operator = %op_name, error = %type_error, "Type error in native string function");
            Err(type_error)
        }
    }
}

// Native function for string concatenation: (string.concat s1 s2 ...)
fn native_string_concat(args: Vec<Expr>) -> Result<Expr, LispError> {
    trace!("Executing native string function: concat");
    let mut result = String::new();
    for (i, arg) in args.iter().enumerate() {
        let s = extract_string(arg, &format!("string.concat (arg {})", i + 1))?;
        result.push_str(&s);
    }
    Ok(Expr::String(result))
}

// Native function for string reversal: (string.reverse s)
fn native_string_reverse(args: Vec<Expr>) -> Result<Expr, LispError> {
    trace!("Executing native string function: reverse");
    if args.len() != 1 {
        let msg = format!("string.reverse expects 1 argument, got {}", args.len());
        error!("{}", msg);
        return Err(LispError::ArityMismatch(msg));
    }
    let s = extract_string(&args[0], "string.reverse")?;
    let reversed_s: String = s.chars().rev().collect();
    Ok(Expr::String(reversed_s))
}

// Native function for string length: (string.len s)
fn native_string_len(args: Vec<Expr>) -> Result<Expr, LispError> {
    trace!("Executing native string function: len");
    if args.len() != 1 {
        let msg = format!("string.len expects 1 argument, got {}", args.len());
        error!("{}", msg);
        return Err(LispError::ArityMismatch(msg));
    }
    let s = extract_string(&args[0], "string.len")?;
    Ok(Expr::Number(s.len() as f64))
}

// Native function for converting string to uppercase: (string.to-upper s)
fn native_string_to_upper(args: Vec<Expr>) -> Result<Expr, LispError> {
    trace!("Executing native string function: to-upper");
    if args.len() != 1 {
        let msg = format!("string.to-upper expects 1 argument, got {}", args.len());
        error!("{}", msg);
        return Err(LispError::ArityMismatch(msg));
    }
    let s = extract_string(&args[0], "string.to-upper")?;
    Ok(Expr::String(s.to_uppercase()))
}

// Native function for converting string to lowercase: (string.to-lower s)
fn native_string_to_lower(args: Vec<Expr>) -> Result<Expr, LispError> {
    trace!("Executing native string function: to-lower");
    if args.len() != 1 {
        let msg = format!("string.to-lower expects 1 argument, got {}", args.len());
        error!("{}", msg);
        return Err(LispError::ArityMismatch(msg));
    }
    let s = extract_string(&args[0], "string.to-lower")?;
    Ok(Expr::String(s.to_lowercase()))
}

// Native function for trimming whitespace: (string.trim s)
fn native_string_trim(args: Vec<Expr>) -> Result<Expr, LispError> {
    trace!("Executing native string function: trim");
    if args.len() != 1 {
        let msg = format!("string.trim expects 1 argument, got {}", args.len());
        error!("{}", msg);
        return Err(LispError::ArityMismatch(msg));
    }
    let s = extract_string(&args[0], "string.trim")?;
    Ok(Expr::String(s.trim().to_string()))
}

/// Creates the `string` module with its associated functions.
pub fn create_string_module() -> Expr {
    trace!("Creating string module");
    let string_env_rc = Environment::new(); // Modules have their own environment
    
    // Scope the mutable borrow so it's dropped before string_env_rc is moved
    {
        let mut string_env_borrowed = string_env_rc.borrow_mut();
        let functions_to_define = HashMap::from([
            (
                "concat".to_string(), // Name within the module
            Expr::NativeFunction(NativeFunction {
                name: "string.concat".to_string(), // Unique name for debugging
                func: native_string_concat,
            }),
        ),
        (
            "reverse".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "string.reverse".to_string(),
                func: native_string_reverse,
            }),
        ),
        (
            "len".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "string.len".to_string(),
                func: native_string_len,
            }),
        ),
        (
            "to-upper".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "string.to-upper".to_string(),
                func: native_string_to_upper,
            }),
        ),
        (
            "to-lower".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "string.to-lower".to_string(),
                func: native_string_to_lower,
            }),
        ),
        (
            "trim".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "string.trim".to_string(),
                func: native_string_trim,
            }),
        ),
    ]);

    for (name, func_expr) in functions_to_define {
            string_env_borrowed.define(name, func_expr);
        }
    } // string_env_borrowed is dropped here

    Expr::Module(LispModule {
        path: PathBuf::from("builtin:string"), // Conventional path for built-in modules
        env: string_env_rc, // Now string_env_rc can be moved
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::eval::eval;
    use crate::engine::parser::parse_expr;
    use crate::logging::init_test_logging;

    // Helper to evaluate a Lisp string in an environment
    fn eval_str(code: &str, env: Rc<RefCell<Environment>>) -> Result<Expr, LispError> {
        let parse_result = parse_expr(code);
        let (remaining, parsed_expr) = match parse_result {
            Ok((rem, expr)) => (rem, expr),
            Err(e) => panic!("Test parse error for code '{}': {}", code, e),
        };

        if !remaining.is_empty() {
            panic!(
                "Unexpected remaining input after parsing in test for code '{}': {}",
                code, remaining
            );
        }
        eval(&parsed_expr, env)
    }

    // Helper to create an environment where string functions are directly callable
    // using "string.<function_name>" symbols, for ease of testing.
    fn env_with_testable_string_functions() -> Rc<RefCell<Environment>> {
        init_test_logging();
        let env = Environment::new_with_prelude();
        let string_module_expr = create_string_module();

        if let Expr::Module(module_data) = string_module_expr {
            // Use the new public method to get bindings
            for (fn_name_in_module, func_expr) in module_data.env.borrow().get_all_bindings() {
                // Define functions like "string.concat", "string.len" in the test environment
                env.borrow_mut().define(
                    format!("string.{}", fn_name_in_module),
                    func_expr.clone(),
                );
            }
        } else {
            panic!("create_string_module did not return a LispModule");
        }
        env
    }

    #[test]
    fn test_string_concat() {
        let env = env_with_testable_string_functions();
        let result = eval_str(r#"(string.concat "hello" " " "world")"#, env.clone()).unwrap();
        assert_eq!(result, Expr::String("hello world".to_string()));

        let result_empty_args = eval_str(r#"(string.concat)"#, env.clone()).unwrap();
        assert_eq!(result_empty_args, Expr::String("".to_string()));

        let result_single_arg = eval_str(r#"(string.concat "test")"#, env.clone()).unwrap();
        assert_eq!(result_single_arg, Expr::String("test".to_string()));
        
        let err_type = eval_str(r#"(string.concat "a" 1)"#, env).unwrap_err();
        assert!(matches!(err_type, LispError::TypeError { expected, .. } if expected == "String"));
    }

    #[test]
    fn test_string_reverse() {
        let env = env_with_testable_string_functions();
        let result = eval_str(r#"(string.reverse "hello")"#, env.clone()).unwrap();
        assert_eq!(result, Expr::String("olleh".to_string()));

        let result_empty_str = eval_str(r#"(string.reverse "")"#, env.clone()).unwrap();
        assert_eq!(result_empty_str, Expr::String("".to_string()));

        let err_arity = eval_str(r#"(string.reverse "a" "b")"#, env.clone()).unwrap_err();
        assert!(matches!(err_arity, LispError::ArityMismatch(_)));

        let err_type = eval_str(r#"(string.reverse 123)"#, env).unwrap_err();
        assert!(matches!(err_type, LispError::TypeError { expected, .. } if expected == "String"));
    }

    #[test]
    fn test_string_len() {
        let env = env_with_testable_string_functions();
        let result = eval_str(r#"(string.len "hello")"#, env.clone()).unwrap();
        assert_eq!(result, Expr::Number(5.0));

        let result_empty_str = eval_str(r#"(string.len "")"#, env.clone()).unwrap();
        assert_eq!(result_empty_str, Expr::Number(0.0));

        let err_arity = eval_str(r#"(string.len "a" "b")"#, env.clone()).unwrap_err();
        assert!(matches!(err_arity, LispError::ArityMismatch(_)));

        let err_type = eval_str(r#"(string.len 123)"#, env).unwrap_err();
        assert!(matches!(err_type, LispError::TypeError { expected, .. } if expected == "String"));
    }

    #[test]
    fn test_string_to_upper() {
        let env = env_with_testable_string_functions();
        let result = eval_str(r#"(string.to-upper "hello World 123")"#, env.clone()).unwrap();
        assert_eq!(result, Expr::String("HELLO WORLD 123".to_string()));
        
        let err_arity = eval_str(r#"(string.to-upper)"#, env.clone()).unwrap_err();
        assert!(matches!(err_arity, LispError::ArityMismatch(_)));

        let err_type = eval_str(r#"(string.to-upper 1)"#, env).unwrap_err();
        assert!(matches!(err_type, LispError::TypeError { expected, .. } if expected == "String"));
    }

    #[test]
    fn test_string_to_lower() {
        let env = env_with_testable_string_functions();
        let result = eval_str(r#"(string.to-lower "Hello WORLD 123")"#, env.clone()).unwrap();
        assert_eq!(result, Expr::String("hello world 123".to_string()));

        let err_arity = eval_str(r#"(string.to-lower)"#, env.clone()).unwrap_err();
        assert!(matches!(err_arity, LispError::ArityMismatch(_)));

        let err_type = eval_str(r#"(string.to-lower 1)"#, env).unwrap_err();
        assert!(matches!(err_type, LispError::TypeError { expected, .. } if expected == "String"));
    }

    #[test]
    fn test_string_trim() {
        let env = env_with_testable_string_functions();
        let result = eval_str(r#"(string.trim "  hello world  ")"#, env.clone()).unwrap();
        assert_eq!(result, Expr::String("hello world".to_string()));

        let result_no_trim_needed = eval_str(r#"(string.trim "hello")"#, env.clone()).unwrap();
        assert_eq!(result_no_trim_needed, Expr::String("hello".to_string()));
        
        let result_empty_after_trim = eval_str(r#"(string.trim "   ")"#, env.clone()).unwrap();
        assert_eq!(result_empty_after_trim, Expr::String("".to_string()));

        let err_arity = eval_str(r#"(string.trim)"#, env.clone()).unwrap_err();
        assert!(matches!(err_arity, LispError::ArityMismatch(_)));
        
        let err_type = eval_str(r#"(string.trim 1)"#, env).unwrap_err();
        assert!(matches!(err_type, LispError::TypeError { expected, .. } if expected == "String"));
    }
}
