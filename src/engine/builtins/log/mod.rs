use crate::engine::ast::{Expr, LispModule, NativeFunction};
use crate::engine::env::Environment;
use crate::engine::eval::LispError;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{instrument, trace};

// Helper function for log/info and log/error
fn _format_log_message_and_write(args: Vec<Expr>, writer: fn(&str)) -> Result<Expr, LispError> {
    if args.is_empty() {
        writer(""); // Print a newline (or empty string, println! adds newline)
        return Ok(Expr::String("".to_string()));
    }

    let format_str_expr = &args[0];
    let format_str = match format_str_expr {
        Expr::String(s) => s,
        _ => {
            // If the first arg isn't a string, treat all args as items to be space-joined.
            let output: Vec<String> = args.iter().map(|arg| arg.to_lisp_string()).collect();
            let result_string = output.join(" ");
            writer(&result_string);
            return Ok(Expr::String(result_string));
        }
    };

    let mut arg_iter = args.iter().skip(1);
    let mut result_string = String::new();
    let mut last_pos = 0;

    while let Some(percent_s_pos) = format_str[last_pos..].find("%s") {
        let current_segment_start = last_pos + percent_s_pos;
        result_string.push_str(&format_str[last_pos..current_segment_start]);

        if let Some(arg_to_print) = arg_iter.next() {
            result_string.push_str(&arg_to_print.to_lisp_string());
        } else {
            result_string.push_str("%s"); // Not enough args, append %s literally
        }
        last_pos = current_segment_start + 2; // Move past "%s"
    }
    result_string.push_str(&format_str[last_pos..]);

    writer(&result_string);
    Ok(Expr::String(result_string))
}

#[instrument(skip(args), ret, err)]
pub fn native_log_info(args: Vec<Expr>) -> Result<Expr, LispError> {
    trace!("Executing native 'log/info' function");
    _format_log_message_and_write(args, |s| println!("{}", s))
}

#[instrument(skip(args), ret, err)]
pub fn native_log_error(args: Vec<Expr>) -> Result<Expr, LispError> {
    trace!("Executing native 'log/error' function");
    _format_log_message_and_write(args, |s| eprintln!("{}", s))
}

pub fn create_log_module() -> Expr {
    trace!("Creating log module");
    let log_env_rc = Environment::new();
    let functions_to_define = HashMap::from([
        (
            "info".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "info".to_string(),
                func: native_log_info,
            }),
        ),
        (
            "error".to_string(),
            Expr::NativeFunction(NativeFunction {
                name: "error".to_string(),
                func: native_log_error,
            }),
        ),
    ]);

    {
        let mut log_env_borrowed = log_env_rc.borrow_mut();
        for (name, expr) in functions_to_define {
            log_env_borrowed.define(name, expr);
        }
    }

    Expr::Module(LispModule {
        path: PathBuf::from("builtin:log"),
        env: log_env_rc,
    })
}

#[cfg(test)]
mod tests {
    use super::{create_log_module, native_log_error, native_log_info};
    use crate::engine::ast::Expr;
    use crate::logging::init_test_logging;

    #[test]
    fn test_native_log_info_no_args() {
        init_test_logging();
        assert_eq!(native_log_info(vec![]), Ok(Expr::String("".to_string())));
        // Expected output to stdout: "" (empty line)
    }

    #[test]
    fn test_native_log_info_single_arg_no_format() {
        init_test_logging();
        // If first arg is not a string with %s, it's treated as a value to print
        let args = vec![Expr::Number(123.0)];
        assert_eq!(native_log_info(args), Ok(Expr::String("123".to_string())));
    }

    #[test]
    fn test_native_log_info_single_string_arg_no_format() {
        init_test_logging();
        let args = vec![Expr::String("hello".to_string())];
        assert_eq!(native_log_info(args), Ok(Expr::String("hello".to_string())));
    }

    #[test]
    fn test_native_log_info_multiple_args_no_format_first_arg_not_string() {
        init_test_logging();
        let args = vec![
            Expr::Number(1.0), // First arg not a string
            Expr::Symbol("world".to_string()),
            Expr::Number(42.0),
        ];
        assert_eq!(
            native_log_info(args),
            Ok(Expr::String("1 world 42".to_string()))
        );
    }

    #[test]
    fn test_native_log_info_multiple_args_no_format_first_arg_string() {
        init_test_logging();
        let args = vec![
            Expr::String("Hello".to_string()), // First arg is string, but no %s
            Expr::Symbol("world".to_string()),
            Expr::Number(42.0),
        ];
        // If first arg is string but no %s, it's treated as format string.
        // Subsequent args are used if %s appears. Here, they are ignored.
        assert_eq!(native_log_info(args), Ok(Expr::String("Hello".to_string())));
    }

    #[test]
    fn test_native_log_info_interpolation() {
        init_test_logging();
        let args = vec![
            Expr::String("Value: %s and %s".to_string()),
            Expr::Number(1.0),
            Expr::Bool(true),
        ];
        assert_eq!(
            native_log_info(args),
            Ok(Expr::String("Value: 1 and true".to_string()))
        );
    }

    #[test]
    fn test_native_log_error_no_args() {
        init_test_logging();
        assert_eq!(native_log_error(vec![]), Ok(Expr::String("".to_string())));
    }

    #[test]
    fn test_native_log_error_single_arg_no_format() {
        init_test_logging();
        let args = vec![Expr::String("ERROR".to_string())];
        assert_eq!(
            native_log_error(args),
            Ok(Expr::String("ERROR".to_string()))
        );
    }

    #[test]
    fn test_native_log_error_multiple_args_interpolation() {
        init_test_logging();
        let args = vec![
            Expr::String("Error: %s failed with %s".to_string()),
            Expr::Symbol("something".to_string()),
            Expr::Number(101.0),
        ];
        assert_eq!(
            native_log_error(args),
            Ok(Expr::String("Error: something failed with 101".to_string()))
        );
    }
}
