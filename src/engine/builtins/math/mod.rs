use crate::engine::ast::{Expr, LispModule, NativeFunction};
use crate::engine::env::Environment;
use crate::engine::eval::LispError;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{error, trace};

// Helper function, not public
fn extract_number(expr: &Expr, op_name: &str) -> Result<f64, LispError> {
    match expr {
        Expr::Number(n) => Ok(*n),
        _ => {
            let type_error = LispError::TypeError {
                expected: "Number".to_string(),
                found: format!("{:?}", expr),
            };
            error!(operator = %op_name, error = %type_error, "Type error in native function");
            Err(type_error)
        }
    }
}

#[tracing::instrument(skip(args), ret, err)]
pub fn native_add(args: Vec<Expr>) -> Result<Expr, LispError> {
    trace!("Executing native '+' function");
    let mut sum = 0.0;
    if args.is_empty() {
        // Standard behavior for (+) is 0
        return Ok(Expr::Number(0.0));
    }
    for arg in args {
        sum += extract_number(&arg, "+")?;
    }
    Ok(Expr::Number(sum))
}

#[tracing::instrument(skip(args), ret, err)]
pub fn native_equals(args: Vec<Expr>) -> Result<Expr, LispError> {
    trace!("Executing native '=' function for numeric equality");
    if args.len() < 2 {
        // In many Lisps, (=) is true, (= x) is true.
        // For numeric comparison, typically at least two args are expected.
        // Let's require at least two for numeric comparison for now.
        // Or, one could define different equality predicates (eq?, eql?, equal?).
        let arity_error = LispError::ArityMismatch(format!(
            "Native '=' expects at least 2 arguments for numeric comparison, got {}",
            args.len()
        ));
        error!(error = %arity_error, "Arity error in native '='");
        return Err(arity_error);
    }

    let first_val = extract_number(&args[0], "=")?;
    for arg_expr in args.iter().skip(1) {
        if first_val != extract_number(arg_expr, "=")? {
            return Ok(Expr::Bool(false));
        }
    }
    Ok(Expr::Bool(true))
}

#[tracing::instrument(skip(args), ret, err)]
pub fn native_multiply(args: Vec<Expr>) -> Result<Expr, LispError> {
    trace!("Executing native '*' function");
    let mut product = 1.0;
    if args.is_empty() {
        // Standard behavior for (*) is 1
        return Ok(Expr::Number(1.0));
    }
    for arg in args {
        product *= extract_number(&arg, "*")?;
    }
    Ok(Expr::Number(product))
}

#[tracing::instrument(skip(args), ret, err)]
pub fn native_subtract(args: Vec<Expr>) -> Result<Expr, LispError> {
    trace!("Executing native '-' function");
    if args.is_empty() {
        let arity_error =
            LispError::ArityMismatch("Native '-' expects at least 1 argument, got 0".to_string());
        error!(error = %arity_error, "Arity error in native '-'");
        return Err(arity_error);
    }

    let first_val = extract_number(&args[0], "-")?;

    if args.len() == 1 {
        // Negation: (- x)
        return Ok(Expr::Number(-first_val));
    }

    // Subtraction: (- x y z ...)
    let mut result = first_val;
    for arg_expr in args.iter().skip(1) {
        result -= extract_number(arg_expr, "-")?;
    }
    Ok(Expr::Number(result))
}

#[tracing::instrument(skip(args), ret, err)]
pub fn native_divide(args: Vec<Expr>) -> Result<Expr, LispError> {
    trace!("Executing native '/' function");
    if args.is_empty() {
        let arity_error =
            LispError::ArityMismatch("Native '/' expects at least 1 argument, got 0".to_string());
        error!(error = %arity_error, "Arity error in native '/'");
        return Err(arity_error);
    }

    let first_val = extract_number(&args[0], "/")?;

    if args.len() == 1 {
        // Reciprocal: (/ x)
        if first_val == 0.0 {
            let div_zero_error = LispError::DivisionByZero(
                "Division by zero in native '/' (reciprocal of 0)".to_string(),
            );
            error!(error = %div_zero_error, "Division by zero error in native '/'");
            return Err(div_zero_error);
        }
        return Ok(Expr::Number(1.0 / first_val));
    }

    // Division: (/ x y z ...)
    let mut result = first_val;
    for (i, arg_expr) in args.iter().skip(1).enumerate() {
        let divisor = extract_number(arg_expr, "/")?;
        if divisor == 0.0 {
            let div_zero_error = LispError::DivisionByZero(format!(
                "Division by zero in native '/' (argument {})",
                i + 2 // +1 for skip, +1 for 1-based indexing
            ));
            error!(error = %div_zero_error, "Division by zero error in native '/'");
            return Err(div_zero_error);
        }
        result /= divisor;
    }
    Ok(Expr::Number(result))
}

// Helper macro to generate comparison functions
macro_rules! define_comparison_fn {
    ($fn_name:ident, $op_str:expr, $op:tt) => {
        #[tracing::instrument(skip(args), ret, err)]
        pub fn $fn_name(args: Vec<Expr>) -> Result<Expr, LispError> {
            trace!("Executing native '{}' function", $op_str);
            if args.len() != 2 {
                let arity_error = LispError::ArityMismatch(format!(
                    "Native '{}' expects exactly 2 arguments, got {}",
                    $op_str,
                    args.len()
                ));
                error!(error = %arity_error, "Arity error in native '{}'", $op_str);
                return Err(arity_error);
            }
            let lhs = extract_number(&args[0], $op_str)?;
            let rhs = extract_number(&args[1], $op_str)?;
            Ok(Expr::Bool(lhs $op rhs))
        }
    };
}

define_comparison_fn!(native_less_than, "<", <);
define_comparison_fn!(native_greater_than, ">", >);
define_comparison_fn!(native_less_than_or_equal, "<=", <=);
define_comparison_fn!(native_greater_than_or_equal, ">=", >=);


pub fn create_math_module() -> Expr {
    trace!("Creating math module");
    let math_env_rc = Environment::new();
    let functions_to_define = HashMap::from([
        (
            "+".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "+".to_string(),
                func: native_add,
            }),
        ),
        (
            "=".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "=".to_string(),
                func: native_equals,
            }),
        ),
        (
            "*".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "*".to_string(),
                func: native_multiply,
            }),
        ),
        (
            "-".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "-".to_string(),
                func: native_subtract,
            }),
        ),
        (
            "/".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "/".to_string(),
                func: native_divide,
            }),
        ),
        (
            "<".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "<".to_string(),
                func: native_less_than,
            }),
        ),
        (
            ">".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: ">".to_string(),
                func: native_greater_than,
            }),
        ),
        (
            "<=".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "<=".to_string(),
                func: native_less_than_or_equal,
            }),
        ),
        (
            ">=".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: ">=".to_string(),
                func: native_greater_than_or_equal,
            }),
        ),
    ]);

    {
        let mut math_env_borrowed = math_env_rc.borrow_mut();
        for (name, expr) in functions_to_define {
            math_env_borrowed.define(name, expr);
        }
    }

    Expr::Module(LispModule {
        path: PathBuf::from("builtin:math"),
        env: math_env_rc,
    })
}

#[cfg(test)]
mod tests {
    use super::*; // Imports native_add, native_equals, native_multiply, extract_number, create_math_module
    use crate::engine::ast::{Expr, NativeFunction};
    use crate::engine::env::Environment;
    use crate::engine::eval::{LispError, eval};
    use crate::logging::init_test_logging;
    // Rc is not used in these tests

    // Tests for native functions (math specific)
    #[test]
    fn test_native_add_simple() {
        init_test_logging();
        let env = Environment::new(); // Use blank env for this specific test setup
        env.borrow_mut().define(
            "+".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "+".to_string(),
                func: native_add,
            }),
        );
        // (+ 1 2)
        let expr = Expr::List(vec![
            Expr::Symbol("+".to_string()),
            Expr::Number(1.0),
            Expr::Number(2.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(3.0)));
    }

    #[test]
    fn test_native_add_multiple_args() {
        init_test_logging();
        let env = Environment::new(); // Use blank env
        env.borrow_mut().define(
            "+".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "+".to_string(),
                func: native_add,
            }),
        );
        // (+ 1 2 3 4)
        let expr = Expr::List(vec![
            Expr::Symbol("+".to_string()),
            Expr::Number(1.0),
            Expr::Number(2.0),
            Expr::Number(3.0),
            Expr::Number(4.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(10.0)));
    }

    #[test]
    fn test_native_add_no_args() {
        init_test_logging();
        let env = Environment::new(); // Use blank env
        env.borrow_mut().define(
            "+".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "+".to_string(),
                func: native_add,
            }),
        );
        // (+)
        let expr = Expr::List(vec![Expr::Symbol("+".to_string())]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(0.0)));
    }

    #[test]
    fn test_native_add_type_error() {
        init_test_logging();
        let env = Environment::new(); // Use blank env
        env.borrow_mut().define(
            "+".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "+".to_string(),
                func: native_add,
            }),
        );
        // (+ 1 true)
        let expr = Expr::List(vec![
            Expr::Symbol("+".to_string()),
            Expr::Number(1.0),
            Expr::Bool(true), // Not a number
        ]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::TypeError {
                expected: "Number".to_string(),
                found: "Bool(true)".to_string()
            })
        );
    }

    #[test]
    fn test_native_equals_true() {
        init_test_logging();
        let env = Environment::new(); // Use blank env
        env.borrow_mut().define(
            "=".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "=".to_string(),
                func: native_equals,
            }),
        );
        // (= 5 5.0)
        let expr = Expr::List(vec![
            Expr::Symbol("=".to_string()),
            Expr::Number(5.0),
            Expr::Number(5.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Bool(true)));
    }

    #[test]
    fn test_native_equals_false() {
        init_test_logging();
        let env = Environment::new(); // Use blank env
        env.borrow_mut().define(
            "=".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "=".to_string(),
                func: native_equals,
            }),
        );
        // (= 5 6)
        let expr = Expr::List(vec![
            Expr::Symbol("=".to_string()),
            Expr::Number(5.0),
            Expr::Number(6.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Bool(false)));
    }

    #[test]
    fn test_native_equals_multiple_true() {
        init_test_logging();
        let env = Environment::new(); // Use blank env
        env.borrow_mut().define(
            "=".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "=".to_string(),
                func: native_equals,
            }),
        );
        // (= 3 3 3 3)
        let expr = Expr::List(vec![
            Expr::Symbol("=".to_string()),
            Expr::Number(3.0),
            Expr::Number(3.0),
            Expr::Number(3.0),
            Expr::Number(3.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Bool(true)));
    }

    #[test]
    fn test_native_equals_multiple_false() {
        init_test_logging();
        let env = Environment::new(); // Use blank env
        env.borrow_mut().define(
            "=".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "=".to_string(),
                func: native_equals,
            }),
        );
        // (= 3 3 4 3)
        let expr = Expr::List(vec![
            Expr::Symbol("=".to_string()),
            Expr::Number(3.0),
            Expr::Number(3.0),
            Expr::Number(4.0),
            Expr::Number(3.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Bool(false)));
    }

    #[test]
    fn test_native_equals_arity_error_too_few() {
        init_test_logging();
        let env = Environment::new(); // Use blank env
        env.borrow_mut().define(
            "=".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "=".to_string(),
                func: native_equals,
            }),
        );
        // (= 5)
        let expr = Expr::List(vec![Expr::Symbol("=".to_string()), Expr::Number(5.0)]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::ArityMismatch(
                "Native '=' expects at least 2 arguments for numeric comparison, got 1".to_string()
            ))
        );
    }

    #[test]
    fn test_native_equals_type_error() {
        init_test_logging();
        let env = Environment::new(); // Use blank env
        env.borrow_mut().define(
            "=".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "=".to_string(),
                func: native_equals,
            }),
        );
        // (= 5 nil)
        let expr = Expr::List(vec![
            Expr::Symbol("=".to_string()),
            Expr::Number(5.0),
            Expr::Nil, // Not a number
        ]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::TypeError {
                expected: "Number".to_string(),
                found: "Nil".to_string()
            })
        );
    }

    // Tests for native_multiply
    #[test]
    fn test_native_multiply_simple() {
        init_test_logging();
        let env = Environment::new_with_prelude(); // Uses prelude which now includes *
        // (* 2 3)
        let expr = Expr::List(vec![
            Expr::Symbol("*".to_string()),
            Expr::Number(2.0),
            Expr::Number(3.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(6.0)));
    }

    #[test]
    fn test_native_multiply_multiple_args() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (* 1 2 3 4)
        let expr = Expr::List(vec![
            Expr::Symbol("*".to_string()),
            Expr::Number(1.0),
            Expr::Number(2.0),
            Expr::Number(3.0),
            Expr::Number(4.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(24.0)));
    }

    #[test]
    fn test_native_multiply_no_args() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (*)
        let expr = Expr::List(vec![Expr::Symbol("*".to_string())]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(1.0)));
    }

    #[test]
    fn test_native_multiply_one_arg() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (* 5)
        let expr = Expr::List(vec![Expr::Symbol("*".to_string()), Expr::Number(5.0)]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(5.0)));
    }

    #[test]
    fn test_native_multiply_with_zero() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (* 5 0 2)
        let expr = Expr::List(vec![
            Expr::Symbol("*".to_string()),
            Expr::Number(5.0),
            Expr::Number(0.0),
            Expr::Number(2.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(0.0)));
    }

    #[test]
    fn test_native_multiply_type_error() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (* 2 true)
        let expr = Expr::List(vec![
            Expr::Symbol("*".to_string()),
            Expr::Number(2.0),
            Expr::Bool(true), // Not a number
        ]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::TypeError {
                expected: "Number".to_string(),
                found: "Bool(true)".to_string()
            })
        );
    }

    // Tests for native_subtract
    #[test]
    fn test_native_subtract_simple() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (- 5 2)
        let expr = Expr::List(vec![
            Expr::Symbol("-".to_string()),
            Expr::Number(5.0),
            Expr::Number(2.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(3.0)));
    }

    #[test]
    fn test_native_subtract_multiple_args() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (- 10 1 2 3)
        let expr = Expr::List(vec![
            Expr::Symbol("-".to_string()),
            Expr::Number(10.0),
            Expr::Number(1.0),
            Expr::Number(2.0),
            Expr::Number(3.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(4.0)));
    }

    #[test]
    fn test_native_subtract_negation() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (- 5)
        let expr = Expr::List(vec![Expr::Symbol("-".to_string()), Expr::Number(5.0)]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(-5.0)));
    }

    #[test]
    fn test_native_subtract_no_args_error() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (-)
        let expr = Expr::List(vec![Expr::Symbol("-".to_string())]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::ArityMismatch(
                "Native '-' expects at least 1 argument, got 0".to_string()
            ))
        );
    }

    #[test]
    fn test_native_subtract_type_error() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (- 10 true)
        let expr = Expr::List(vec![
            Expr::Symbol("-".to_string()),
            Expr::Number(10.0),
            Expr::Bool(true), // Not a number
        ]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::TypeError {
                expected: "Number".to_string(),
                found: "Bool(true)".to_string()
            })
        );
    }

    #[test]
    fn test_native_subtract_type_error_negation() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (- true)
        let expr = Expr::List(vec![Expr::Symbol("-".to_string()), Expr::Bool(true)]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::TypeError {
                expected: "Number".to_string(),
                found: "Bool(true)".to_string()
            })
        );
    }

    // Tests for native_divide
    #[test]
    fn test_native_divide_simple() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (/ 10 2)
        let expr = Expr::List(vec![
            Expr::Symbol("/".to_string()),
            Expr::Number(10.0),
            Expr::Number(2.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(5.0)));
    }

    #[test]
    fn test_native_divide_multiple_args() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (/ 100 2 5)
        let expr = Expr::List(vec![
            Expr::Symbol("/".to_string()),
            Expr::Number(100.0),
            Expr::Number(2.0),
            Expr::Number(5.0),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(10.0)));
    }

    #[test]
    fn test_native_divide_reciprocal() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (/ 4)
        let expr = Expr::List(vec![Expr::Symbol("/".to_string()), Expr::Number(4.0)]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(0.25)));
    }

    #[test]
    fn test_native_divide_reciprocal_zero_error() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (/ 0)
        let expr = Expr::List(vec![Expr::Symbol("/".to_string()), Expr::Number(0.0)]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::DivisionByZero(
                "Division by zero in native '/' (reciprocal of 0)".to_string()
            ))
        );
    }

    #[test]
    fn test_native_divide_by_zero_error() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (/ 10 0)
        let expr = Expr::List(vec![
            Expr::Symbol("/".to_string()),
            Expr::Number(10.0),
            Expr::Number(0.0),
        ]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::DivisionByZero(
                "Division by zero in native '/' (argument 2)".to_string()
            ))
        );
    }

    #[test]
    fn test_native_divide_by_zero_multiple_args_error() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (/ 10 2 0 5)
        let expr = Expr::List(vec![
            Expr::Symbol("/".to_string()),
            Expr::Number(10.0),
            Expr::Number(2.0),
            Expr::Number(0.0),
            Expr::Number(5.0),
        ]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::DivisionByZero(
                "Division by zero in native '/' (argument 3)".to_string()
            ))
        );
    }

    #[test]
    fn test_native_divide_no_args_error() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (/)
        let expr = Expr::List(vec![Expr::Symbol("/".to_string())]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::ArityMismatch(
                "Native '/' expects at least 1 argument, got 0".to_string()
            ))
        );
    }

    #[test]
    fn test_native_divide_type_error() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (/ 10 true)
        let expr = Expr::List(vec![
            Expr::Symbol("/".to_string()),
            Expr::Number(10.0),
            Expr::Bool(true), // Not a number
        ]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::TypeError {
                expected: "Number".to_string(),
                found: "Bool(true)".to_string()
            })
        );
    }

    #[test]
    fn test_native_divide_type_error_reciprocal() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        // (/ true)
        let expr = Expr::List(vec![Expr::Symbol("/".to_string()), Expr::Bool(true)]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::TypeError {
                expected: "Number".to_string(),
                found: "Bool(true)".to_string()
            })
        );
    }

    // Helper macro for testing comparison functions
    macro_rules! test_comparison_fn {
        ($test_name:ident, $op_str:expr, $native_fn:ident, $lhs:expr, $rhs:expr, $expected:expr) => {
            #[test]
            fn $test_name() {
                init_test_logging();
                let env = Environment::new_with_prelude(); // Uses prelude
                let expr = Expr::List(vec![
                    Expr::Symbol($op_str.to_string()),
                    Expr::Number($lhs),
                    Expr::Number($rhs),
                ]);
                assert_eq!(eval(&expr, env), Ok(Expr::Bool($expected)));
            }
        };
        // For type error tests
        ($test_name:ident, $op_str:expr, $native_fn:ident, $lhs:expr, $rhs_expr:expr, expected_err_found: $expected_err_val:expr) => {
            #[test]
            fn $test_name() {
                init_test_logging();
                let env = Environment::new_with_prelude();
                let expr = Expr::List(vec![
                    Expr::Symbol($op_str.to_string()),
                    Expr::Number($lhs),
                    $rhs_expr, // e.g. Expr::Bool(true)
                ]);
                assert_eq!(
                    eval(&expr, env),
                    Err(LispError::TypeError {
                        expected: "Number".to_string(),
                        found: $expected_err_val.to_string()
                    })
                );
            }
        };
        // For arity error tests
        ($test_name:ident, $op_str:expr, $native_fn:ident, arity_args: $args_val:expr, expected_len: $len_val:expr) => {
            #[test]
            fn $test_name() {
                init_test_logging();
                let env = Environment::new_with_prelude();
                let mut expr_args = vec![Expr::Symbol($op_str.to_string())];
                for arg_val in $args_val { // Use $args_val here
                    expr_args.push(Expr::Number(arg_val));
                }
                let expr = Expr::List(expr_args);
                assert_eq!(
                    eval(&expr, env),
                    Err(LispError::ArityMismatch(format!(
                        "Native '{}' expects exactly 2 arguments, got {}",
                        $op_str,
                        $len_val // Use $len_val here
                    )))
                );
            }
        };
    }

    // Tests for native_less_than (<)
    test_comparison_fn!(test_native_less_than_true, "<", native_less_than, 2.0, 5.0, true);
    test_comparison_fn!(test_native_less_than_false_equal, "<", native_less_than, 5.0, 5.0, false);
    test_comparison_fn!(test_native_less_than_false_greater, "<", native_less_than, 5.0, 2.0, false);
    test_comparison_fn!(test_native_less_than_type_error, "<", native_less_than, 2.0, Expr::Bool(true), expected_err_found: "Bool(true)");
    test_comparison_fn!(test_native_less_than_arity_too_few, "<", native_less_than, arity_args: [2.0], expected_len: 1);
    test_comparison_fn!(test_native_less_than_arity_too_many, "<", native_less_than, arity_args: [2.0, 3.0, 4.0], expected_len: 3);

    // Tests for native_greater_than (>)
    test_comparison_fn!(test_native_greater_than_true, ">", native_greater_than, 5.0, 2.0, true);
    test_comparison_fn!(test_native_greater_than_false_equal, ">", native_greater_than, 5.0, 5.0, false);
    test_comparison_fn!(test_native_greater_than_false_less, ">", native_greater_than, 2.0, 5.0, false);
    test_comparison_fn!(test_native_greater_than_type_error, ">", native_greater_than, 5.0, Expr::String("s".to_string()), expected_err_found: "String(\"s\")");
    test_comparison_fn!(test_native_greater_than_arity_too_few, ">", native_greater_than, arity_args: [2.0], expected_len: 1);
    test_comparison_fn!(test_native_greater_than_arity_too_many, ">", native_greater_than, arity_args: [2.0, 3.0, 4.0], expected_len: 3);

    // Tests for native_less_than_or_equal (<=)
    test_comparison_fn!(test_native_less_than_or_equal_true_less, "<=", native_less_than_or_equal, 2.0, 5.0, true);
    test_comparison_fn!(test_native_less_than_or_equal_true_equal, "<=", native_less_than_or_equal, 5.0, 5.0, true);
    test_comparison_fn!(test_native_less_than_or_equal_false_greater, "<=", native_less_than_or_equal, 5.0, 2.0, false);
    test_comparison_fn!(test_native_less_than_or_equal_type_error, "<=", native_less_than_or_equal, 2.0, Expr::Nil, expected_err_found: "Nil");
    test_comparison_fn!(test_native_less_than_or_equal_arity_too_few, "<=", native_less_than_or_equal, arity_args: [2.0], expected_len: 1);
    test_comparison_fn!(test_native_less_than_or_equal_arity_too_many, "<=", native_less_than_or_equal, arity_args: [2.0, 3.0, 4.0], expected_len: 3);

    // Tests for native_greater_than_or_equal (>=)
    test_comparison_fn!(test_native_greater_than_or_equal_true_greater, ">=", native_greater_than_or_equal, 5.0, 2.0, true);
    test_comparison_fn!(test_native_greater_than_or_equal_true_equal, ">=", native_greater_than_or_equal, 5.0, 5.0, true);
    test_comparison_fn!(test_native_greater_than_or_equal_false_less, ">=", native_greater_than_or_equal, 2.0, 5.0, false);
    test_comparison_fn!(test_native_greater_than_or_equal_type_error, ">=", native_greater_than_or_equal, 5.0, Expr::Symbol("sym".to_string()), expected_err_found: "Symbol(\"sym\")");
    test_comparison_fn!(test_native_greater_than_or_equal_arity_too_few, ">=", native_greater_than_or_equal, arity_args: [2.0], expected_len: 1);
    test_comparison_fn!(test_native_greater_than_or_equal_arity_too_many, ">=", native_greater_than_or_equal, arity_args: [2.0, 3.0, 4.0], expected_len: 3);
}
