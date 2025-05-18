use crate::engine::ast::{Expr, LispFunction};
use crate::engine::env::Environment;
use crate::engine::eval::LispError; // eval_let needs to return LispError
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
    if crate::engine::special_forms::is_special_form(&var_name) {
        error!(attempted_keyword = %var_name, "Attempted to bind a reserved keyword using 'let'");
        return Err(LispError::ReservedKeyword(var_name));
    }

    debug!(variable_name = %var_name, value_expression = ?value_expr, "'let' binding");
    // Note: We need to call back into the main eval function here.
    // This requires `crate::engine::eval::eval` to be accessible.
    let evaluated_value = crate::engine::eval::eval(value_expr, Rc::clone(&env))?;

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
                if crate::engine::special_forms::is_special_form(name) {
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

    let condition_result = crate::engine::eval::eval(condition_expr, Rc::clone(&env))?;
    debug!(?condition_result, "Evaluated 'if' condition");

    match condition_result {
        Expr::Bool(false) | Expr::Nil => {
            // Condition is false or nil, evaluate else-branch or return Nil
            if let Some(else_expr) = else_expr_opt {
                trace!("Condition is false-y, evaluating else-branch");
                crate::engine::eval::eval(else_expr, env)
            } else {
                trace!("Condition is false-y, no else-branch, returning Nil");
                Ok(Expr::Nil)
            }
        }
        _ => {
            // Condition is truthy (anything not false or Nil)
            trace!("Condition is truthy, evaluating then-branch");
            crate::engine::eval::eval(then_expr, env)
        }
    }
}

use std::fs;
use std::path::PathBuf;

#[tracing::instrument(skip(args, _env), fields(args = ?args), ret, err)] // _env as it's not used directly for require's own logic
pub fn eval_require(args: &[Expr], _env: Rc<RefCell<Environment>>) -> Result<Expr, LispError> {
    trace!("Executing 'require' special form");
    if args.len() != 1 {
        let msg = format!(
            "'require' expects 1 argument (path string or symbol), got {}",
            args.len()
        );
        error!("{}", msg);
        return Err(LispError::ArityMismatch(msg));
    }

    let path_specifier_expr = &args[0];
    let mut relative_path_str = match path_specifier_expr {
        Expr::String(s) => s.clone(),
        Expr::Symbol(s) => s.clone(), // Treat symbol name as path directly
        _ => {
            let msg = format!(
                "'require' argument must be a string or symbol, found {:?}",
                path_specifier_expr
            );
            error!("{}", msg);
            return Err(LispError::TypeError {
                expected: "String or Symbol path".to_string(),
                found: format!("{:?}", path_specifier_expr),
            });
        }
    };

    if !relative_path_str.ends_with(".lisp") {
        relative_path_str.push_str(".lisp");
    }

    let current_dir = std::env::current_dir().map_err(|e| LispError::ModuleIoError {
        path: PathBuf::from(relative_path_str.clone()), // Use relative path for error context here
        kind: e.kind(),
        message: e.to_string(),
    })?;
    let mut absolute_path = current_dir;
    absolute_path.push(&relative_path_str);

    let canonical_path = match fs::canonicalize(&absolute_path) {
        Ok(p) => p,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Err(LispError::ModuleNotFound(absolute_path));
            } else {
                return Err(LispError::ModuleIoError {
                    path: absolute_path,
                    kind: e.kind(),
                    message: e.to_string(),
                });
            }
        }
    };

    debug!(path_specifier = ?path_specifier_expr, resolved_path = %canonical_path.display(), "Path for 'require'");

    // Check cache
    {
        // Accessing thread_local storage
        let cached_module = crate::MODULE_CACHE.with(|cache_cell| {
            let cache = cache_cell.borrow();
            cache.get(&canonical_path).cloned() // Clone if found
        });
        if let Some(module) = cached_module {
            trace!(path = %canonical_path.display(), "Module found in cache");
            return Ok(module);
        }
    } // End of cache check block (borrow of MODULE_CACHE is released)

    // Load and evaluate module
    let content = match fs::read_to_string(&canonical_path) {
        Ok(c) => c,
        Err(e) => {
            return Err(LispError::ModuleIoError {
                path: canonical_path,
                kind: e.kind(),
                message: e.to_string(),
            });
        }
    };

    let module_env = Environment::new_with_prelude();
    let mut current_module_input: &str = &content;

    loop {
        current_module_input = current_module_input.trim_start();
        if current_module_input.is_empty() {
            break;
        }
        match crate::engine::parser::parse_expr(current_module_input) {
            Ok((remaining, ast)) => {
                if let Err(e) = crate::engine::eval::eval(&ast, Rc::clone(&module_env)) {
                    error!(module_path = %canonical_path.display(), error = %e, "Error evaluating expression in module");
                    return Err(LispError::ModuleLoadError {
                        path: canonical_path,
                        source: Box::new(e),
                    });
                }
                current_module_input = remaining;
            }
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                if !current_module_input.is_empty() {
                    let parse_err_msg = format!(
                        "Parsing Error in module '{}': {:?}",
                        canonical_path.display(),
                        e
                    );
                    error!("{}", parse_err_msg);
                    return Err(LispError::ModuleLoadError {
                        path: canonical_path,
                        source: Box::new(LispError::Evaluation(format!(
                            "Module parsing error: {}",
                            parse_err_msg
                        ))),
                    });
                }
                break;
            }
            Err(nom::Err::Incomplete(_)) => {
                let msg = format!(
                    "Parsing incomplete in module '{}': More input needed.",
                    canonical_path.display()
                );
                error!("{}", msg);
                return Err(LispError::ModuleLoadError {
                    path: canonical_path,
                    source: Box::new(LispError::Evaluation(msg)),
                });
            }
        }
    }

    let new_module = Expr::Module(crate::engine::ast::LispModule {
        path: canonical_path.clone(),
        env: module_env,
    });

    // Add to cache
    {
        crate::MODULE_CACHE.with(|cache_cell| {
            let mut cache = cache_cell.borrow_mut();
            cache.insert(canonical_path.clone(), new_module.clone());
        });
        trace!(path = %canonical_path.display(), "Module loaded and cached");
    }

    Ok(new_module)
}

// Native Rust functions callable from Lisp (the "prelude" functions)

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

// Future built-in functions will go here.

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

#[tracing::instrument(skip(args), ret, err)]
pub fn native_log_info(args: Vec<Expr>) -> Result<Expr, LispError> {
    trace!("Executing native 'log/info' function");
    _format_log_message_and_write(args, |s| println!("{}", s))
}

#[tracing::instrument(skip(args), ret, err)]
pub fn native_log_error(args: Vec<Expr>) -> Result<Expr, LispError> {
    trace!("Executing native 'log/error' function");
    _format_log_message_and_write(args, |s| eprintln!("{}", s))
}

#[cfg(test)]
mod tests {
    use super::{
        native_add, native_equals, native_log_error, native_log_info, native_module_ref,
    }; // Updated imports
    use crate::engine::ast::{Expr, LispFunction, LispModule, NativeFunction}; // Added NativeFunction & LispModule
    use crate::engine::env::Environment;
    use crate::engine::eval::{LispError, eval}; // Need main eval for testing integration
    use crate::logging::init_test_logging; // Use new logging setup
    use std::path::PathBuf; // Import PathBuf for tests
    use std::rc::Rc; // For Environment

    #[test]
    fn eval_let_binding() {
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
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
            Ok(Expr::Function(LispFunction {
                params,
                body,
                closure,
            })) => {
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
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
        let env = Environment::new();
        // (quote 10)
        let expr = Expr::List(vec![Expr::Symbol("quote".to_string()), Expr::Number(10.0)]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(10.0)));
    }

    #[test]
    fn eval_quote_list() {
        init_test_logging();
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
        init_test_logging();
        let env = Environment::new();
        // (quote ())
        let expr = Expr::List(vec![Expr::Symbol("quote".to_string()), Expr::List(vec![])]);
        assert_eq!(eval(&expr, env), Ok(Expr::List(vec![])));
    }

    #[test]
    fn eval_quote_nested_list() {
        init_test_logging();
        let env = Environment::new();
        // (quote (a (b c)))
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
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
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
        init_test_logging();
        let env = Environment::new();
        env.borrow_mut()
            .define("cond-var".to_string(), Expr::Bool(true));
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
        init_test_logging();
        let env = Environment::new();
        // (if true)
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
        init_test_logging();
        let env = Environment::new();
        env.borrow_mut()
            .define("then-val".to_string(), Expr::Number(100.0));
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
        init_test_logging();
        let env = Environment::new();
        env.borrow_mut()
            .define("else-val".to_string(), Expr::Number(200.0));
        // (if false then-val else-val) ; then-val is undefined
        let expr = Expr::List(vec![
            Expr::Symbol("if".to_string()),
            Expr::Bool(false),
            Expr::Symbol("then-val".to_string()), // This should not be evaluated
            Expr::Symbol("else-val".to_string()),
        ]);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(200.0)));
    }

    // Tests for native functions
    // These tests manually add the native functions to the environment.
    // Later, a prelude mechanism will do this automatically.

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

    // Tests for native_log_info and native_log_error
    // These tests primarily check that the functions can be called and return Nil.
    // Direct stdout/stderr capture is not implemented here.

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
        assert_eq!(native_log_info(args), Ok(Expr::String("1 world 42".to_string())));
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
        assert_eq!(native_log_info(args), Ok(Expr::String("Value: 1 and true".to_string())));
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
        assert_eq!(native_log_error(args), Ok(Expr::String("ERROR".to_string())));
    }

    #[test]
    fn test_native_log_error_multiple_args_interpolation() {
        init_test_logging();
        let args = vec![
            Expr::String("Error: %s failed with %s".to_string()),
            Expr::Symbol("something".to_string()),
            Expr::Number(101.0),
        ];
        assert_eq!(native_log_error(args), Ok(Expr::String("Error: something failed with 101".to_string())));
    }

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
