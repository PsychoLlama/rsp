use crate::engine::ast::Expr;
use crate::engine::builtins;
use crate::engine::env::Environment;
use crate::engine::special_forms; // Added for special form constants
use std::cell::RefCell;
use std::rc::Rc;
use thiserror::Error;
use tracing::{debug, error, instrument, trace};

#[derive(Error, Debug, Clone, PartialEq)]
pub enum LispError {
    #[error("Evaluation error: {0}")]
    Evaluation(String),
    #[error("Type error: expected {expected}, found {found}")]
    TypeError { expected: String, found: String },
    #[error("Undefined symbol: {0}")]
    UndefinedSymbol(String),
    #[error("Invalid arguments for operator '{operator}': {message}")]
    InvalidArguments { operator: String, message: String },
    #[error("Arity mismatch: {0}")]
    ArityMismatch(String),
    #[error("Cannot bind reserved keyword: {0}")]
    ReservedKeyword(String),
    #[error("Not a function: {0}")]
    NotAFunction(String),
    // Add more specific errors as the interpreter develops
}

#[instrument(skip(expr, env), fields(expr = ?expr), ret, err)]
pub fn eval(expr: &Expr, env: Rc<RefCell<Environment>>) -> Result<Expr, LispError> {
    trace!("Starting evaluation");
    match expr {
        Expr::Number(_) | Expr::Function(_) | Expr::NativeFunction(_) | Expr::Bool(_) | Expr::Nil => {
            debug!(env = ?env.borrow(), "Evaluating Number, Function, NativeFunction, Bool, or Nil: {:?}", expr);
            Ok(expr.clone()) // These types evaluate to themselves
        }
        Expr::Symbol(s) => {
            debug!(env = ?env.borrow(), symbol_name = %s, "Evaluating Symbol");
            if let Some(value) = env.borrow().get(s) {
                trace!(symbol_name = %s, value = ?value, "Found symbol in environment");
                Ok(value)
            } else {
                error!(symbol_name = %s, "Undefined symbol encountered");
                Err(LispError::UndefinedSymbol(s.clone()))
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
                Expr::Symbol(s) if s == special_forms::LET => {
                    builtins::eval_let(&list[1..], Rc::clone(&env))
                }
                Expr::Symbol(s) if s == special_forms::QUOTE => {
                    builtins::eval_quote(&list[1..])
                }
                Expr::Symbol(s) if s == special_forms::FN => {
                    builtins::eval_fn(&list[1..], Rc::clone(&env))
                }
                Expr::Symbol(s) if s == special_forms::IF => {
                    builtins::eval_if(&list[1..], Rc::clone(&env))
                }
                // Attempt to evaluate as a function call
                _ => {
                    trace!("First element is not a known special form, attempting function call");
                    // 1. Evaluate the first element of the list (the potential function)
                    let func_expr = eval(first_form, Rc::clone(&env))?;

                    match func_expr {
                        Expr::Function(lisp_fn) => {
                            debug!(function = ?lisp_fn, "Attempting to call LispFunction");

                            // 2. Evaluate the arguments
                            let mut evaluated_args = Vec::new();
                            for arg_expr in &list[1..] {
                                evaluated_args.push(eval(arg_expr, Rc::clone(&env))?);
                            }

                            // 3. Check arity
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

                            // 4. Create a new environment for the function call, enclosed by the function's closure
                            let call_env = Environment::new_enclosed(Rc::clone(&lisp_fn.closure));
                            trace!(?call_env, "Created new environment for function call");

                            // 5. Bind parameters to arguments in the new environment
                            for (param_name, arg_value) in
                                lisp_fn.params.iter().zip(evaluated_args.iter())
                            {
                                call_env
                                    .borrow_mut()
                                    .define(param_name.clone(), arg_value.clone());
                                trace!(param = %param_name, value = ?arg_value, "Bound parameter in call environment");
                            }

                            // 6. Evaluate the function body in the new environment
                            debug!(body = ?lisp_fn.body, "Evaluating function body");
                            eval(&lisp_fn.body, call_env)
                        }
                        Expr::NativeFunction(native_fn) => {
                            debug!(native_function_name = %native_fn.name, "Attempting to call NativeFunction");
                            // 2. Evaluate the arguments (already done for LispFunction, repeat for consistency or refactor)
                            let mut evaluated_args = Vec::new();
                            for arg_expr in &list[1..] {
                                evaluated_args.push(eval(arg_expr, Rc::clone(&env))?);
                            }
                            // 3. Call the native Rust function
                            trace!(args = ?evaluated_args, "Calling native function with evaluated arguments");
                            (native_fn.func)(evaluated_args)
                        }
                        _ => {
                            error!(non_function_expr = ?first_form, evaluated_to = ?func_expr, "Attempted to call a non-function or non-native-function");
                            Err(LispError::NotAFunction(format!(
                                "Expected a Lisp function or a native function, but found: {:?}",
                                func_expr
                            )))
                        }
                    }
                }
            }
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
        let expr = Expr::List(vec![Expr::Symbol("x".to_string()), Expr::Number(1.0), Expr::Number(2.0)]);
        assert_eq!(
            eval(&expr, env),
            Err(LispError::NotAFunction(
                "Expected a Lisp function or a native function, but found: Number(10.0)".to_string()
            ))
        );
    }

    #[test]
    fn eval_call_non_function_number() {
        init_test_logging();
        let env = Environment::new();
        // (1 2 3) - trying to call a number
        let expr = Expr::List(vec![Expr::Number(1.0), Expr::Number(2.0), Expr::Number(3.0)]);
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
        let call_expr = Expr::List(vec![
            Expr::Symbol("my-fn".to_string()),
            Expr::Number(10.0),
        ]);
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
                Expr::List(vec![Expr::Symbol("a".to_string()), Expr::Symbol("b".to_string())]),
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
                Expr::List(vec![Expr::Symbol("x".to_string()), Expr::Symbol("y".to_string())]),
                Expr::Symbol("x".to_string()),
            ]),
        ]);
        eval(&define_fn_expr, Rc::clone(&env)).unwrap();

        // (my-fn 10) - too few args
        let call_expr = Expr::List(vec![
            Expr::Symbol("my-fn".to_string()),
            Expr::Number(10.0),
        ]);
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
        env.borrow_mut().define("captured_val".to_string(), Expr::Number(100.0));

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
        env.borrow_mut().define("captured_val".to_string(), Expr::Number(999.0));

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
        
        env.borrow_mut().define("outer_val".to_string(), Expr::Number(50.0));

        let define_generator_expr = Expr::List(vec![
            Expr::Symbol("let".to_string()),
            Expr::Symbol("fn_generator".to_string()),
            Expr::List(vec![ // (fn (param1) ...)
                Expr::Symbol("fn".to_string()),
                Expr::List(vec![Expr::Symbol("param1".to_string())]),
                Expr::List(vec![ // (fn (param2) outer_val)
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
        env.borrow_mut().define("outer_val".to_string(), Expr::Number(777.0));

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

        eval(&Expr::List(vec![ // (let f (fn () 10))
            Expr::Symbol("let".to_string()),
            Expr::Symbol("f".to_string()),
            Expr::List(vec![
                Expr::Symbol("fn".to_string()),
                Expr::List(vec![]),
                Expr::Number(10.0)
            ])
        ]), Rc::clone(&env)).unwrap();

        eval(&Expr::List(vec![ // (let g (fn () (f)))
            Expr::Symbol("let".to_string()),
            Expr::Symbol("g".to_string()),
            Expr::List(vec![
                Expr::Symbol("fn".to_string()),
                Expr::List(vec![]),
                Expr::List(vec![Expr::Symbol("f".to_string())]) // Call f
            ])
        ]), Rc::clone(&env)).unwrap();

        // Redefine f
        eval(&Expr::List(vec![ // (let f (fn () 20))
            Expr::Symbol("let".to_string()),
            Expr::Symbol("f".to_string()),
            Expr::List(vec![
                Expr::Symbol("fn".to_string()),
                Expr::List(vec![]),
                Expr::Number(20.0)
            ])
        ]), Rc::clone(&env)).unwrap();
        
        // Call g
        let call_g_expr = Expr::List(vec![Expr::Symbol("g".to_string())]);
        assert_eq!(eval(&call_g_expr, env), Ok(Expr::Number(20.0))); // g calls the f from its closure, which has been updated
    }
}
