    use crate::engine::ast::{Expr, LispFunction, LispModule};
    use crate::engine::env::Environment;
    use crate::engine::eval::{eval as main_eval, LispError}; // Renamed to avoid conflict
    use crate::engine::parser;
    use crate::engine::special_forms as special_form_constants; // For LET, QUOTE, FN, IF, REQUIRE, is_special_form
    use crate::MODULE_CACHE;
    use std::cell::RefCell;
    use std::fs;
    use std::path::PathBuf;
    use std::rc::Rc;
    use tracing::{debug, error, instrument, trace};
    
    #[instrument(skip(args, env), fields(args = ?args), ret, err)]
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
        if special_form_constants::is_special_form(&var_name) {
            error!(attempted_keyword = %var_name, "Attempted to bind a reserved keyword using 'let'");
            return Err(LispError::ReservedKeyword(var_name));
        }
    
        debug!(variable_name = %var_name, value_expression = ?value_expr, "'let' binding");
        let evaluated_value = main_eval(value_expr, Rc::clone(&env))?;
    
        env.borrow_mut()
            .define(var_name.clone(), evaluated_value.clone());
        debug!(variable_name = %var_name, value = ?evaluated_value, "Defined variable in environment using 'let'");
        Ok(evaluated_value)
    }
    
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
    
    #[instrument(skip(args, _env), fields(args = ?args), ret, err)]
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
            Expr::Symbol(s) => s.clone(),
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
            path: PathBuf::from(relative_path_str.clone()),
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
    
        {
            let cached_module = MODULE_CACHE.with(|cache_cell| {
                let cache = cache_cell.borrow();
                cache.get(&canonical_path).cloned()
            });
            if let Some(module) = cached_module {
                trace!(path = %canonical_path.display(), "Module found in cache");
                return Ok(module);
            }
        }
    
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
            match parser::parse_expr(current_module_input) {
                Ok((remaining, ast)) => {
                    if let Err(e) = main_eval(&ast, Rc::clone(&module_env)) {
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
    
        let new_module = Expr::Module(LispModule {
            path: canonical_path.clone(),
            env: module_env,
        });
    
        {
            MODULE_CACHE.with(|cache_cell| {
                let mut cache = cache_cell.borrow_mut();
                cache.insert(canonical_path.clone(), new_module.clone());
            });
            trace!(path = %canonical_path.display(), "Module loaded and cached");
        }
    
        Ok(new_module)
    }
    
    #[cfg(test)]
    mod tests {
        use super::*; // Imports the special form eval functions
        use crate::engine::ast::Expr; // LispFunction is in super, LispModule is in super
        use crate::engine::env::Environment;
        use crate::engine::eval::{eval, LispError}; // The main eval for setting up tests
        use crate::logging::init_test_logging;
        // PathBuf is not used directly in these tests.
        use std::rc::Rc;
    
        #[test]
        fn eval_let_binding() {
            init_test_logging();
            let env = Environment::new();
            let let_expr = Expr::List(vec![
                Expr::Symbol("let".to_string()),
                Expr::Symbol("x".to_string()),
                Expr::Number(10.0),
            ]);
            assert_eq!(eval(&let_expr, Rc::clone(&env)), Ok(Expr::Number(10.0)));
            assert_eq!(env.borrow().get("x"), Some(Expr::Number(10.0)));
    
            let x_sym = Expr::Symbol("x".to_string());
            assert_eq!(eval(&x_sym, Rc::clone(&env)), Ok(Expr::Number(10.0)));
        }
    
        #[test]
        fn eval_let_binding_evaluates_value() {
            init_test_logging();
            let env = Environment::new();
            env.borrow_mut().define("y".to_string(), Expr::Number(5.0));
            let let_expr = Expr::List(vec![
                Expr::Symbol("let".to_string()),
                Expr::Symbol("x".to_string()),
                Expr::Symbol("y".to_string()),
            ]);
            assert_eq!(eval(&let_expr, Rc::clone(&env)), Ok(Expr::Number(5.0)));
            assert_eq!(env.borrow().get("x"), Some(Expr::Number(5.0)));
        }
    
        #[test]
        fn eval_let_arity_error_too_few_args() {
            init_test_logging();
            let env = Environment::new();
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
            let let_expr = Expr::List(vec![
                Expr::Symbol("let".to_string()),
                Expr::Number(10.0), 
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
            let expr = Expr::List(vec![
                Expr::Symbol("let".to_string()),
                Expr::Symbol("let".to_string()),
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
            let expr = Expr::List(vec![
                Expr::Symbol("let".to_string()),
                Expr::Symbol("quote".to_string()),
                Expr::Number(10.0),
            ]);
            assert_eq!(
                eval(&expr, env),
                Err(LispError::ReservedKeyword("quote".to_string()))
            );
        }
    
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
