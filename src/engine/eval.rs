use crate::engine::ast::Expr;
// builtins direct import might not be needed if all calls are fully qualified to submodules
use crate::engine::env::Environment;
use crate::engine::special_forms as special_form_constants; // Renamed for clarity
use std::cell::RefCell;
use std::rc::Rc;
use thiserror::Error;
use tracing::{debug, error, instrument, trace};

#[derive(Error, Debug, Clone, PartialEq)]
pub enum LispError {
    #[error("Evaluation error: {0}")]
    #[allow(dead_code)] // May be used in future development
    Evaluation(String),
    #[error("Type error: expected {expected}, found {found}")]
    TypeError { expected: String, found: String },
    #[error("Undefined symbol: {0}")]
    UndefinedSymbol(String),
    #[error("Invalid arguments for operator '{operator}': {message}")]
    #[allow(dead_code)] // May be used in future development
    InvalidArguments { operator: String, message: String },
    #[error("Arity mismatch: {0}")]
    ArityMismatch(String),
    #[error("Cannot bind reserved keyword: {0}")]
    ReservedKeyword(String),
    #[error("Not a function: {0}")]
    NotAFunction(String),
    #[error("Module not found: {0:?}")]
    ModuleNotFound(std::path::PathBuf),
    #[error("Error loading module '{path:?}': {source}")]
    ModuleLoadError {
        path: std::path::PathBuf,
        source: Box<LispError>,
    },
    #[error("I/O error for module '{path:?}': kind: {kind:?}, message: {message}")]
    ModuleIoError {
        path: std::path::PathBuf,
        kind: std::io::ErrorKind,
        message: String,
    },
    #[error("Symbol '{0}' is not a module, cannot access members.")]
    NotAModule(String),
    #[error("Member '{member}' not found in module '{module}'.")]
    MemberNotFoundInModule { module: String, member: String },
    #[error("Division by zero: {0}")]
    DivisionByZero(String),
    #[error("Value error: {0}")]
    ValueError(String),
    // Add more specific errors as the interpreter develops
}

#[instrument(skip(expr, env), fields(expr = ?expr), ret, err)]
pub fn eval(expr: &Expr, env: Rc<RefCell<Environment>>) -> Result<Expr, LispError> {
    trace!("Starting evaluation");
    match expr {
        Expr::Number(_)
        | Expr::Function(_)
        | Expr::NativeFunction(_)
        | Expr::Bool(_)
        | Expr::Nil
        | Expr::String(_) // Added String to self-evaluating types
        | Expr::Module(_) => {
            debug!(env = ?env.borrow(), "Evaluating Number, Function, NativeFunction, Bool, Nil, String, or Module: {:?}", expr);
            Ok(expr.clone()) // These types evaluate to themselves
        }
        Expr::Symbol(s) => {
            debug!(env = ?env.borrow(), symbol_name = %s, "Evaluating Symbol");
            if s.contains('/') {
                let parts: Vec<&str> = s.splitn(2, '/').collect();
                if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
                    let module_var_name = parts[0];
                    let member_name = parts[1];

                    match env.borrow().get(module_var_name) {
                        Some(Expr::Module(lisp_module)) => {
                            trace!(module_variable = %module_var_name, member_name, "Accessing member of module held by variable.");
                            lisp_module.env.borrow().get(member_name).ok_or_else(|| {
                                error!(module_name = %module_var_name, member_name, "Member not found in module.");
                                LispError::MemberNotFoundInModule {
                                    module: module_var_name.to_string(),
                                    member: member_name.to_string(),
                                }
                            })
                        }
                        Some(other_expr) => {
                            error!(variable_name = %module_var_name, value = ?other_expr, "Variable is not a module, cannot access member.");
                            Err(LispError::NotAModule(module_var_name.to_string()))
                        }
                        None => {
                            // If the module variable itself is not found, it's an UndefinedSymbol error for the module variable.
                            error!(module_variable_name = %module_var_name, "Module variable not found for member access.");
                            Err(LispError::UndefinedSymbol(module_var_name.to_string()))
                        }
                    }
                } else {
                    // Invalid format like "foo/" or "/bar", treat as a normal (likely undefined) symbol lookup.
                    // This maintains consistency: if it's not a valid path, it's just a symbol.
                    trace!(symbol_name = %s, "Symbol with '/' has invalid module/member format, treating as regular symbol.");
                    env.borrow().get(s).ok_or_else(|| {
                        error!(symbol_name = %s, "Undefined symbol (invalid path format) encountered");
                        LispError::UndefinedSymbol(s.clone())
                    })
                }
            } else {
                // Regular symbol lookup (no '/')
                env.borrow().get(s).ok_or_else(|| {
                    error!(symbol_name = %s, "Undefined regular symbol encountered");
                    LispError::UndefinedSymbol(s.clone())
                })
            }
        }
        Expr::List(list) => {
            debug!(env = ?env.borrow(), "Evaluating List: {:?}", list);
            if list.is_empty() {
                trace!("List is empty, evaluating to empty list");
                return Ok(Expr::List(Vec::new())); // Empty list evaluates to itself
            }

            // Handle special forms and function calls
            let first_form = &list[0];
            match first_form {
                Expr::Symbol(s) if s == special_form_constants::LET => {
                    crate::engine::builtins::special_forms::eval_let(&list[1..], Rc::clone(&env))
                }
                Expr::Symbol(s) if s == special_form_constants::QUOTE => {
                    crate::engine::builtins::special_forms::eval_quote(&list[1..])
                }
                Expr::Symbol(s) if s == special_form_constants::FN => {
                    crate::engine::builtins::special_forms::eval_fn(&list[1..], Rc::clone(&env))
                }
                Expr::Symbol(s) if s == special_form_constants::IF => {
                    crate::engine::builtins::special_forms::eval_if(&list[1..], Rc::clone(&env))
                }
                Expr::Symbol(s) if s == special_form_constants::REQUIRE => {
                    crate::engine::builtins::special_forms::eval_require(&list[1..], Rc::clone(&env))
                }
                // Attempt to evaluate as a function call
                _ => {
                    trace!("First element is not a known special form, attempting function call");

                    // 1. Resolve or evaluate the first element of the list to get the function expression.
                    let func_expr_to_call = match first_form {
                        Expr::Symbol(s) => {
                            if s.contains('/') {
                                let parts: Vec<&str> = s.splitn(2, '/').collect();
                                if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
                                    let module_candidate_name = parts[0];
                                    let member_name = parts[1];

                                    // Attempt 1: Evaluate module_candidate_name as a variable.
                                    // If it resolves to a module, use that.
                                    match eval(&Expr::Symbol(module_candidate_name.to_string()), Rc::clone(&env)) {
                                        Ok(Expr::Module(lisp_module)) => {
                                            trace!(variable_name = %module_candidate_name, member_name, "Variable resolved to module. Looking up member.");
                                            match lisp_module.env.borrow().get(member_name) {
                                                Some(member_expr) => Ok(member_expr),
                                                None => Err(LispError::MemberNotFoundInModule {
                                                    module: format!("variable '{}' (bound to module '{}')", module_candidate_name, lisp_module.path.display()),
                                                    member: member_name.to_string(),
                                                }),
                                            }
                                        }
                                        Ok(other_value) => {
                                            // Variable resolved, but not to a module. This is an error for path-like access.
                                            // Or, it could be that module_candidate_name is a direct module name like "math"
                                            // and not a variable. Fall through to Attempt 2.
                                            trace!(variable_name = %module_candidate_name, value = ?other_value, "Variable did not resolve to a module or was not found. Trying as direct module name.");
                                            // Attempt 2: Treat module_candidate_name as a direct module name (string).
                                            match env.borrow().get(module_candidate_name) {
                                                Some(Expr::Module(lisp_module)) => {
                                                    trace!(module_name = %module_candidate_name, member_name, "Found direct module. Looking up member.");
                                                    match lisp_module.env.borrow().get(member_name) {
                                                        Some(member_expr) => Ok(member_expr),
                                                        None => Err(LispError::MemberNotFoundInModule {
                                                            module: module_candidate_name.to_string(),
                                                            member: member_name.to_string(),
                                                        }),
                                                    }
                                                }
                                                Some(_) => Err(LispError::NotAModule(module_candidate_name.to_string())),
                                                None => Err(LispError::UndefinedSymbol(s.clone())), // Original 'x/member' symbol is problematic
                                            }
                                        }
                                        Err(LispError::UndefinedSymbol(_)) => {
                                            // module_candidate_name is not a defined variable. Try as direct module name.
                                            trace!(module_name = %module_candidate_name, member_name, "Symbol for module part not found. Trying as direct module name.");
                                            match env.borrow().get(module_candidate_name) {
                                                Some(Expr::Module(lisp_module)) => {
                                                    match lisp_module.env.borrow().get(member_name) {
                                                        Some(member_expr) => Ok(member_expr),
                                                        None => Err(LispError::MemberNotFoundInModule {
                                                            module: module_candidate_name.to_string(),
                                                            member: member_name.to_string(),
                                                        }),
                                                    }
                                                }
                                                Some(_) => Err(LispError::NotAModule(module_candidate_name.to_string())),
                                                None => Err(LispError::UndefinedSymbol(s.clone())), // Original 'module/member' symbol is problematic
                                            }
                                        }
                                        Err(e) => Err(e), // Other evaluation error for the module candidate part
                                    }
                                } else {
                                    // Symbol contains '/' but not in valid module/member format. Treat as regular symbol.
                                    trace!(symbol_name = %s, "Symbol contains '/' but not a valid module/member path, evaluating as regular symbol");
                                    eval(first_form, Rc::clone(&env))
                                }
                            } else {
                                // Symbol does not contain '/', treat as regular symbol.
                                trace!(symbol_name = %s, "Symbol does not contain '/', evaluating as regular symbol");
                                eval(first_form, Rc::clone(&env))
                            }
                        }
                        _ => {
                            // First form is not a symbol (e.g., a list that evaluates to a function).
                            trace!(?first_form, "First form is not a symbol, evaluating it to get function");
                            eval(first_form, Rc::clone(&env))
                        }
                    }?; // func_expr_to_call is the resolved Expr to be called

                    // 2. Evaluate all arguments
                    let mut evaluated_args = Vec::new();
                    for arg_expr in &list[1..] {
                        evaluated_args.push(eval(arg_expr, Rc::clone(&env))?);
                    }

                    // 3. Apply the function
                    apply(func_expr_to_call, evaluated_args, Rc::clone(&env)) // Pass cloned env
                }
            }
        }
    }
}

/// Applies a function (Lisp or native) to a list of evaluated arguments.
#[instrument(skip(func_expr_to_call, evaluated_args, _calling_env), fields(func = ?func_expr_to_call, args = ?evaluated_args), ret, err)]
fn apply(
    func_expr_to_call: Expr, // Renamed parameter for clarity
    evaluated_args: Vec<Expr>,
    _calling_env: Rc<RefCell<Environment>>, // Use the passed environment, prefixed with _
) -> Result<Expr, LispError> {
    match func_expr_to_call {
        // Use the renamed parameter
        Expr::Function(lisp_fn) => {
            debug!(function = ?lisp_fn, "Applying LispFunction");

            // Check arity
            if evaluated_args.len() != lisp_fn.params.len() {
                error!(
                    expected = lisp_fn.params.len(),
                    got = evaluated_args.len(),
                    "Arity mismatch for function call"
                );
                return Err(LispError::ArityMismatch(format!(
                    "Function expects {} arguments, got {}",
                    lisp_fn.params.len(),
                    evaluated_args.len()
                )));
            }

            // Create a new environment for the function call, enclosed by the function's closure
            let call_env = Environment::new_enclosed(Rc::clone(&lisp_fn.closure));
            trace!(?call_env, "Created new environment for function call");

            // Bind parameters to arguments in the new environment
            for (param_name, arg_value) in lisp_fn.params.iter().zip(evaluated_args.iter()) {
                call_env
                    .borrow_mut()
                    .define(param_name.clone(), arg_value.clone());
                trace!(param = %param_name, value = ?arg_value, "Bound parameter in call environment");
            }

            // Evaluate the function body in the new environment
            debug!(body = ?lisp_fn.body, "Evaluating function body");
            eval(&lisp_fn.body, call_env)
        }
        Expr::NativeFunction(native_fn) => {
            debug!(native_function_name = %native_fn.name, "Applying NativeFunction");
            // Call the native Rust function
            trace!(args = ?evaluated_args, "Calling native function with evaluated arguments");
            (native_fn.func)(evaluated_args)
        }
        _ => {
            error!(evaluated_to = ?func_expr_to_call, "Attempted to call a non-function or non-native-function expression");
            Err(LispError::NotAFunction(format!(
                "Expected a Lisp function or a native function, but found: {:?}",
                func_expr_to_call
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*; // Imports eval, Expr, LispError, Environment, Rc, RefCell
    use crate::logging::init_test_logging; // Use new logging setup

    #[test]
    fn eval_number() {
        init_test_logging();
        let env = Environment::new();
        let expr = Expr::Number(42.0);
        assert_eq!(eval(&expr, env), Ok(Expr::Number(42.0)));
    }

    #[test]
    fn eval_symbol_defined_in_env() {
        init_test_logging();
        let env = Environment::new();
        env.borrow_mut()
            .define("x".to_string(), Expr::Number(100.0));
        let expr = Expr::Symbol("x".to_string());
        assert_eq!(eval(&expr, env), Ok(Expr::Number(100.0)));
    }

    #[test]
    fn eval_symbol_defined_in_outer_env() {
        init_test_logging();
        let outer_env = Environment::new();
        outer_env
            .borrow_mut()
            .define("x".to_string(), Expr::Number(100.0));
        let inner_env = Environment::new_enclosed(outer_env);
        let expr = Expr::Symbol("x".to_string());
        assert_eq!(eval(&expr, inner_env), Ok(Expr::Number(100.0)));
    }

    #[test]
    fn eval_symbol_shadowed() {
        init_test_logging();
        let outer_env = Environment::new();
        outer_env
            .borrow_mut()
            .define("x".to_string(), Expr::Number(100.0));
        let inner_env = Environment::new_enclosed(outer_env.clone());
        inner_env
            .borrow_mut()
            .define("x".to_string(), Expr::Number(200.0)); // Shadow

        let expr = Expr::Symbol("x".to_string());
        assert_eq!(eval(&expr, inner_env), Ok(Expr::Number(200.0)));
        // Ensure outer is not affected by eval call on inner
        assert_eq!(outer_env.borrow().get("x"), Some(Expr::Number(100.0)));
    }

    #[test]
    fn eval_symbol_undefined() {
        init_test_logging();
        let env = Environment::new();
        let expr = Expr::Symbol("my_var".to_string());
        assert_eq!(
            eval(&expr, env),
            Err(LispError::UndefinedSymbol("my_var".to_string()))
        );
    }

    #[test]
    fn eval_empty_list() {
        init_test_logging();
        let env = Environment::new();
        let expr = Expr::List(vec![]);
        assert_eq!(eval(&expr, env), Ok(Expr::List(vec![])));
    }

    #[test]
    fn eval_true_literal() {
        init_test_logging();
        let env = Environment::new();
        let expr = Expr::Bool(true);
        assert_eq!(eval(&expr, env), Ok(Expr::Bool(true)));
    }

    #[test]
    fn eval_false_literal() {
        init_test_logging();
        let env = Environment::new();
        let expr = Expr::Bool(false);
        assert_eq!(eval(&expr, env), Ok(Expr::Bool(false)));
    }

    #[test]
    fn eval_nil_literal() {
        init_test_logging();
        let env = Environment::new();
        let expr = Expr::Nil;
        assert_eq!(eval(&expr, env), Ok(Expr::Nil));
    }

    #[test]
    fn eval_non_empty_list_not_implemented() {
        init_test_logging();
        let env = Environment::new();
        let expr = Expr::List(vec![
            Expr::Symbol("unknown_function".to_string()),
            Expr::Number(1.0),
        ]);
        assert_eq!(
            eval(&expr, env),
            // If "unknown_function" is not defined, eval(first_form) will yield UndefinedSymbol.
            Err(LispError::UndefinedSymbol("unknown_function".to_string()))
        );
    }

    #[test]
    fn eval_call_defined_non_function() {
        init_test_logging();
        let env = Environment::new();
        // (let x 10)
        env.borrow_mut().define("x".to_string(), Expr::Number(10.0));
        // (x 1 2)
        let expr = Expr::List(vec![
            Expr::Symbol("x".to_string()),
            Expr::Number(1.0),
            Expr::Number(2.0),
        ]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::NotAFunction(
                "Expected a Lisp function or a native function, but found: Number(10.0)"
                    .to_string()
            ))
        );
    }

    #[test]
    fn eval_call_non_function_number() {
        init_test_logging();
        let env = Environment::new();
        // (1 2 3) - trying to call a number
        let expr = Expr::List(vec![
            Expr::Number(1.0),
            Expr::Number(2.0),
            Expr::Number(3.0),
        ]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::NotAFunction(
                "Expected a Lisp function or a native function, but found: Number(1.0)".to_string()
            ))
        );
    }

    // Tests for 'fn' and function calls
    #[test]
    fn eval_fn_definition_and_call() {
        init_test_logging();
        let env = Environment::new();
        // (let my-fn (fn (x) x))
        let define_fn_expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Symbol("my-fn".to_string()),
            Expr::List(vec![
                Expr::Symbol("fn".to_string()),
                Expr::List(vec![Expr::Symbol("x".to_string())]),
                Expr::Symbol("x".to_string()),
            ]),
        ]);
        eval(&define_fn_expr, Rc::clone(&env)).unwrap();

        // (my-fn 10)
        let call_expr = Expr::List(vec![Expr::Symbol("my-fn".to_string()), Expr::Number(10.0)]);
        assert_eq!(eval(&call_expr, env), Ok(Expr::Number(10.0)));
    }

    #[test]
    fn eval_fn_call_with_multiple_params() {
        init_test_logging();
        let env = Environment::new();
        // (let add (fn (a b) ???)) ; body needs actual addition, which we don't have yet.
        // For now, let's make a function that just returns its second param.
        // (let my-fn (fn (a b) b))
        let define_fn_expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Symbol("my-fn".to_string()),
            Expr::List(vec![
                Expr::Symbol("fn".to_string()),
                Expr::List(vec![
                    Expr::Symbol("a".to_string()),
                    Expr::Symbol("b".to_string()),
                ]),
                Expr::Symbol("b".to_string()), // Returns the second param
            ]),
        ]);
        eval(&define_fn_expr, Rc::clone(&env)).unwrap();

        // (my-fn 10 20)
        let call_expr = Expr::List(vec![
            Expr::Symbol("my-fn".to_string()),
            Expr::Number(10.0),
            Expr::Number(20.0),
        ]);
        assert_eq!(eval(&call_expr, env), Ok(Expr::Number(20.0)));
    }

    #[test]
    fn eval_fn_call_arity_mismatch_too_few() {
        init_test_logging();
        let env = Environment::new();
        // (let my-fn (fn (x y) x))
        let define_fn_expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Symbol("my-fn".to_string()),
            Expr::List(vec![
                Expr::Symbol("fn".to_string()),
                Expr::List(vec![
                    Expr::Symbol("x".to_string()),
                    Expr::Symbol("y".to_string()),
                ]),
                Expr::Symbol("x".to_string()),
            ]),
        ]);
        eval(&define_fn_expr, Rc::clone(&env)).unwrap();

        // (my-fn 10) - too few args
        let call_expr = Expr::List(vec![Expr::Symbol("my-fn".to_string()), Expr::Number(10.0)]);
        assert_eq!(
            eval(&call_expr, env),
            Err(LispError::ArityMismatch(
                "Function expects 2 arguments, got 1".to_string()
            ))
        );
    }

    #[test]
    fn eval_fn_call_arity_mismatch_too_many() {
        init_test_logging();
        let env = Environment::new();
        // (let my-fn (fn (x) x))
        let define_fn_expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Symbol("my-fn".to_string()),
            Expr::List(vec![
                Expr::Symbol("fn".to_string()),
                Expr::List(vec![Expr::Symbol("x".to_string())]),
                Expr::Symbol("x".to_string()),
            ]),
        ]);
        eval(&define_fn_expr, Rc::clone(&env)).unwrap();

        // (my-fn 10 20) - too many args
        let call_expr = Expr::List(vec![
            Expr::Symbol("my-fn".to_string()),
            Expr::Number(10.0),
            Expr::Number(20.0),
        ]);
        assert_eq!(
            eval(&call_expr, env),
            Err(LispError::ArityMismatch(
                "Function expects 1 arguments, got 2".to_string()
            ))
        );
    }

    #[test]
    fn eval_closure_captures_env() {
        init_test_logging();
        let env = Environment::new();
        // (let y 5)
        env.borrow_mut().define("y".to_string(), Expr::Number(5.0));

        // (let make-adder (fn (x) (fn (z) ???))) ; body needs x+z, use y for now
        // (let make-adder (fn (x) (fn (z) y))) ; inner fn should capture y from outer scope of make-adder
        // This test is a bit tricky without arithmetic. Let's make it simpler:
        // (let captured-val 100)
        // (let my-closure (fn () captured-val))
        // (my-closure) -> 100
        env.borrow_mut()
            .define("captured_val".to_string(), Expr::Number(100.0));

        let define_closure_expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Symbol("my_closure".to_string()),
            Expr::List(vec![
                Expr::Symbol("fn".to_string()),
                Expr::List(vec![]), // No params
                Expr::Symbol("captured_val".to_string()),
            ]),
        ]);
        eval(&define_closure_expr, Rc::clone(&env)).unwrap();

        // Now, let's shadow `captured_val` in the current env to ensure the closure uses its captured one.
        env.borrow_mut()
            .define("captured_val".to_string(), Expr::Number(999.0));

        let call_closure_expr = Expr::List(vec![Expr::Symbol("my_closure".to_string())]);
        assert_eq!(eval(&call_closure_expr, env), Ok(Expr::Number(999.0))); // Expect the captured env to see the update
    }

    #[test]
    fn eval_closure_with_params_and_captured_var() {
        init_test_logging();
        let env = Environment::new();
        // (let adder (fn (x) (fn (y) ???))) ; x+y
        // For now, let's make a function that returns its captured var, ignoring params
        // (let outer-val 50)
        // (let fn-generator (fn (param1) (fn (param2) outer-val)))
        // (let my-fn (fn-generator 10)) ; param1 is 10, not used by inner
        // (my-fn 20) -> 50 ; param2 is 20, not used by inner

        env.borrow_mut()
            .define("outer_val".to_string(), Expr::Number(50.0));

        let define_generator_expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Symbol("fn_generator".to_string()),
            Expr::List(vec![
                // (fn (param1) ...)
                Expr::Symbol("fn".to_string()),
                Expr::List(vec![Expr::Symbol("param1".to_string())]),
                Expr::List(vec![
                    // (fn (param2) outer_val)
                    Expr::Symbol("fn".to_string()),
                    Expr::List(vec![Expr::Symbol("param2".to_string())]),
                    Expr::Symbol("outer_val".to_string()),
                ]),
            ]),
        ]);
        eval(&define_generator_expr, Rc::clone(&env)).unwrap();

        // (let my_fn (fn_generator 10))
        let get_inner_fn_expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Symbol("my_fn".to_string()),
            Expr::List(vec![
                Expr::Symbol("fn_generator".to_string()),
                Expr::Number(10.0), // Argument for param1
            ]),
        ]);
        eval(&get_inner_fn_expr, Rc::clone(&env)).unwrap();

        // Shadow outer_val to ensure closure uses the one from its definition time
        env.borrow_mut()
            .define("outer_val".to_string(), Expr::Number(777.0));

        // (my_fn 20)
        let call_inner_fn_expr = Expr::List(vec![
            Expr::Symbol("my_fn".to_string()),
            Expr::Number(20.0), // Argument for param2
        ]);
        assert_eq!(eval(&call_inner_fn_expr, env), Ok(Expr::Number(777.0))); // Expect the closure to see the updated outer_val
    }

    #[test]
    fn eval_recursive_fn_let_style() {
        init_test_logging();
        let env = Environment::new();
        // (let fact (fn (n) ...)) - this doesn't work for recursion directly with `let`
        // because `fact` is not in scope inside the `fn` body yet.
        // A common way is to use a Y combinator or have `letrec`.
        // For now, we can test if a function can call another function defined in env.
        // (let is-zero (fn (n) (if (= n 0) true false))) ; needs if and =
        // (let dec (fn (n) (- n 1))) ; needs -
        // (let fact (fn (n) (if (is-zero n) 1 (* n (fact (dec n))))))
        // This is too complex without `if` and arithmetic.

        // Simpler: (let f (fn () g)) (let g (fn () 10)) (f) -> error g undefined in f's closure
        // This demonstrates that functions capture their lexical scope.
        // To make `f` call `g` defined later, `g` must be in `f`'s lexical scope at definition.

        // Let's test a mutually recursive-like scenario if `let` redefines.
        // (let f (fn () 10))
        // (let g (fn () (f))) ; g captures current f
        // (let f (fn () 20)) ; new f, g still has old f
        // (g) -> 10

        eval(
            &Expr::List(vec![
                // (let f (fn () 10))
                Expr::Symbol("let".to_string()),
                Expr::Symbol("f".to_string()),
                Expr::List(vec![
                    Expr::Symbol("fn".to_string()),
                    Expr::List(vec![]),
                    Expr::Number(10.0),
                ]),
            ]),
            Rc::clone(&env),
        )
        .unwrap();

        eval(
            &Expr::List(vec![
                // (let g (fn () (f)))
                Expr::Symbol("let".to_string()),
                Expr::Symbol("g".to_string()),
                Expr::List(vec![
                    Expr::Symbol("fn".to_string()),
                    Expr::List(vec![]),
                    Expr::List(vec![Expr::Symbol("f".to_string())]), // Call f
                ]),
            ]),
            Rc::clone(&env),
        )
        .unwrap();

        // Redefine f
        eval(
            &Expr::List(vec![
                // (let f (fn () 20))
                Expr::Symbol("let".to_string()),
                Expr::Symbol("f".to_string()),
                Expr::List(vec![
                    Expr::Symbol("fn".to_string()),
                    Expr::List(vec![]),
                    Expr::Number(20.0),
                ]),
            ]),
            Rc::clone(&env),
        )
        .unwrap();

        // Call g
        let call_g_expr = Expr::List(vec![Expr::Symbol("g".to_string())]);
        assert_eq!(eval(&call_g_expr, env), Ok(Expr::Number(20.0))); // g calls the f from its closure, which has been updated
    }

    #[test]
    fn eval_call_member_on_variable_bound_to_module() {
        init_test_logging();
        let env = Environment::new_with_prelude(); // Prelude includes 'math', 'string' modules

        // (let my-math math)
        let let_expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Symbol("my-math".to_string()),
            Expr::Symbol("math".to_string()), // 'math' is a global symbol bound to the math module
        ]);
        eval(&let_expr, Rc::clone(&env)).expect("Failed to let-bind my-math to math module");

        // (my-math/+ 10 5)
        let call_expr = Expr::List(vec![
            Expr::Symbol("my-math/+".to_string()),
            Expr::Number(10.0),
            Expr::Number(5.0),
        ]);
        assert_eq!(eval(&call_expr, Rc::clone(&env)), Ok(Expr::Number(15.0)));

        // (let s (require 'string))
        let let_s_expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Symbol("s".to_string()),
            Expr::List(vec![
                Expr::Symbol("require".to_string()),
                Expr::List(vec![
                    // 'string
                    Expr::Symbol("quote".to_string()),
                    Expr::Symbol("string".to_string()),
                ]),
            ]),
        ]);
        eval(&let_s_expr, Rc::clone(&env))
            .expect("Failed to let-bind s to string module via require");

        // (s/concat "hello" " " "world")
        let call_s_concat_expr = Expr::List(vec![
            Expr::Symbol("s/concat".to_string()),
            Expr::String("hello".to_string()),
            Expr::String(" ".to_string()),
            Expr::String("world".to_string()),
        ]);
        assert_eq!(
            eval(&call_s_concat_expr, Rc::clone(&env)),
            Ok(Expr::String("hello world".to_string()))
        );
    }

    #[test]
    fn eval_call_member_on_variable_not_a_module() {
        init_test_logging();
        let env = Environment::new_with_prelude();

        // (let my-var 123)
        let let_expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Symbol("my-var".to_string()),
            Expr::Number(123.0),
        ]);
        eval(&let_expr, Rc::clone(&env)).expect("Failed to let-bind my-var");

        // (my-var/foo)
        let call_expr = Expr::List(vec![Expr::Symbol("my-var/foo".to_string())]);

        // This should fail because 'my-var' is a number, not a module.
        // The specific error depends on the resolution path.
        // If 'my-var' is evaluated first, it's not a module.
        // If 'my-var' is treated as a module name, it's not found or not a module.
        // The refined logic should lead to NotAModule if 'my-var' is found but isn't a module.
        // Or UndefinedSymbol for "my-var/foo" if "my-var" is not a module and not a global module name.
        // Given the new logic, eval("my-var") -> Number(123), then it tries get("my-var") as module name.
        // If "my-var" is not a global module, it will be UndefinedSymbol("my-var/foo").
        // If "my-var" *was* a global module (but it's not), it would be NotAModule.
        // The current logic correctly identifies that 'my-var' is bound but not a module.
        let result = eval(&call_expr, Rc::clone(&env));
        assert!(matches!(result, Err(LispError::NotAModule(s)) if s == "my-var"));
    }

    // Tests for module variable access: symbol/member
    #[test]
    fn eval_module_variable_access_symbol() {
        init_test_logging();
        let env = Environment::new();

        // Create a dummy module
        let module_env = Environment::new();
        module_env
            .borrow_mut()
            .define("member_var".to_string(), Expr::Number(123.0));
        let lisp_module = Expr::Module(crate::engine::ast::LispModule {
            path: std::path::PathBuf::from("test_mod"),
            env: module_env,
        });

        // (let m test_mod)
        env.borrow_mut()
            .define("m".to_string(), lisp_module.clone());

        // m/member_var
        let expr = Expr::Symbol("m/member_var".to_string());
        assert_eq!(eval(&expr, Rc::clone(&env)), Ok(Expr::Number(123.0)));
    }

    #[test]
    fn eval_module_variable_access_member_not_found() {
        init_test_logging();
        let env = Environment::new();
        let module_env = Environment::new(); // Empty module
        let lisp_module = Expr::Module(crate::engine::ast::LispModule {
            path: std::path::PathBuf::from("test_mod"),
            env: module_env,
        });
        env.borrow_mut()
            .define("m".to_string(), lisp_module.clone());

        // m/non_existent
        let expr = Expr::Symbol("m/non_existent".to_string());
        assert_eq!(
            eval(&expr, Rc::clone(&env)),
            Err(LispError::MemberNotFoundInModule {
                module: "m".to_string(),
                member: "non_existent".to_string()
            })
        );
    }

    #[test]
    fn eval_module_variable_access_not_a_module() {
        init_test_logging();
        let env = Environment::new();
        env.borrow_mut()
            .define("not_a_module".to_string(), Expr::Number(42.0));

        // not_a_module/member
        let expr = Expr::Symbol("not_a_module/member".to_string());
        assert_eq!(
            eval(&expr, Rc::clone(&env)),
            Err(LispError::NotAModule("not_a_module".to_string()))
        );
    }

    #[test]
    fn eval_module_variable_access_module_var_undefined() {
        init_test_logging();
        let env = Environment::new();

        // undefined_mod/member
        let expr = Expr::Symbol("undefined_mod/member".to_string());
        assert_eq!(
            eval(&expr, Rc::clone(&env)),
            Err(LispError::UndefinedSymbol("undefined_mod".to_string()))
        );
    }

    #[test]
    fn eval_symbol_with_slash_invalid_format() {
        init_test_logging();
        let env = Environment::new();
        // foo/ (empty member name)
        assert_eq!(
            eval(&Expr::Symbol("foo/".to_string()), Rc::clone(&env)),
            Err(LispError::UndefinedSymbol("foo/".to_string()))
        );
        // /bar (empty module name)
        assert_eq!(
            eval(&Expr::Symbol("/bar".to_string()), Rc::clone(&env)),
            Err(LispError::UndefinedSymbol("/bar".to_string()))
        );
         // / (just a slash)
         assert_eq!(
            eval(&Expr::Symbol("/".to_string()), Rc::clone(&env)),
            Err(LispError::UndefinedSymbol("/".to_string()))
        );
    }
}
