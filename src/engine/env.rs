use crate::engine::ast::{Expr, NativeFunction};
use crate::engine::builtins::{native_add, native_equals, native_multiply}; // native_println removed from direct import
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use tracing::{debug, trace};

#[derive(Debug, PartialEq)]
pub struct Environment {
    bindings: HashMap<String, Expr>,
    outer: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    /// Creates a new, empty root environment without any prelude functions.
    #[allow(dead_code)] // This is used by tests in other modules
    pub fn new() -> Rc<RefCell<Self>> {
        debug!("Creating new empty root environment");
        Rc::new(RefCell::new(Environment {
            bindings: HashMap::new(),
            outer: None,
        }))
    }

    /// Creates a new, empty root environment and populates it with prelude functions.
    pub fn new_with_prelude() -> Rc<RefCell<Self>> {
        debug!("Creating new root environment with prelude");
        let env_rc = Rc::new(RefCell::new(Environment {
            bindings: HashMap::new(),
            outer: None,
        }));

        // Define prelude functions
        // Each tuple is (Lisp name, Rust function pointer)
        const PRELUDE_NATIVE_FUNCTIONS: &[(&str, crate::engine::ast::NativeFn)] = &[
            // Shorthand versions
            ("+", native_add),
            ("=", native_equals),
            ("*", native_multiply),
            // Namespaced-like versions (parsed as single symbols)
            ("math/+", native_add),
            ("math/=", native_equals),
            ("math/*", native_multiply),
            // Other builtins
            ("println", crate::engine::builtins::native_println),
        ];

        const MATH_FUNCTION_NAMES: &[&str] = &["+", "=", "*"]; // These are the short names for the math module's internal env

        // Create the math module environment
        let math_module_env = Rc::new(RefCell::new(Environment {
            bindings: HashMap::new(),
            outer: None, 
        }));

        {
            let mut math_env_borrowed = math_module_env.borrow_mut();
            for (name, func) in PRELUDE_NATIVE_FUNCTIONS {
                if MATH_FUNCTION_NAMES.contains(name) {
                    // Define math functions in math module's env
                    math_env_borrowed.define(
                        name.to_string(),
                        Expr::NativeFunction(NativeFunction {
                            name: name.to_string(),
                            func: *func,
                        }),
                    );
                }
            }
        }

        // Create the math module Expr
        let math_module = Expr::Module(crate::engine::ast::LispModule {
            path: std::path::PathBuf::from("builtin:math"), // Virtual path for built-in module
            env: math_module_env,
        });

        // Define functions directly in the root prelude AND define the 'math' module
        {
            let mut root_env_borrowed = env_rc.borrow_mut();
            // Define math module in root prelude
            root_env_borrowed.define("math".to_string(), math_module);

            // Define all prelude functions directly in root prelude for shorthand access
            for (name, func) in PRELUDE_NATIVE_FUNCTIONS {
                root_env_borrowed.define(
                    name.to_string(),
                    Expr::NativeFunction(NativeFunction {
                        name: name.to_string(),
                        func: *func,
                    }),
                );
            }
        }
        trace!(env = ?env_rc.borrow(), "Environment after adding prelude");
        env_rc
    }

    /// Creates a new environment that is enclosed by an outer environment.
    pub fn new_enclosed(outer_env: Rc<RefCell<Environment>>) -> Rc<RefCell<Self>> {
        debug!("Creating new enclosed environment");
        Rc::new(RefCell::new(Environment {
            bindings: HashMap::new(),
            outer: Some(outer_env),
        }))
    }

    /// Defines a new variable or redefines an existing one in the current environment.
    pub fn define(&mut self, name: String, value: Expr) {
        trace!(name = %name, value = ?value, "Defining variable in current environment");
        self.bindings.insert(name, value);
    }

    /// Attempts to retrieve a variable's value from the environment.
    /// If not found in the current environment, it searches in outer environments.
    pub fn get(&self, name: &str) -> Option<Expr> {
        trace!(name = %name, "Attempting to get variable from environment");
        if let Some(value) = self.bindings.get(name) {
            debug!(name = %name, value = ?value, "Found variable in current environment");
            Some(value.clone())
        } else {
            match &self.outer {
                Some(outer_env) => {
                    trace!(name = %name, "Variable not in current environment, checking outer environment");
                    outer_env.borrow().get(name)
                }
                None => {
                    debug!(name = %name, "Variable not found in any environment");
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ast::Expr;
    use crate::logging::init_test_logging; // Use new logging setup

    #[test]
    fn define_and_get_in_root_env() {
        init_test_logging();
        let env = Environment::new(); // Test with a blank environment
        env.borrow_mut().define("x".to_string(), Expr::Number(10.0));
        assert_eq!(env.borrow().get("x"), Some(Expr::Number(10.0)));
    }

    #[test]
    fn get_from_outer_env() {
        init_test_logging();
        let outer_env = Environment::new();
        outer_env
            .borrow_mut()
            .define("x".to_string(), Expr::Number(10.0));

        let inner_env = Environment::new_enclosed(outer_env.clone());
        assert_eq!(inner_env.borrow().get("x"), Some(Expr::Number(10.0)));
    }

    #[test]
    fn define_in_inner_shadows_outer() {
        init_test_logging();
        let outer_env = Environment::new();
        outer_env
            .borrow_mut()
            .define("x".to_string(), Expr::Number(10.0));

        let inner_env = Environment::new_enclosed(outer_env.clone());
        inner_env
            .borrow_mut()
            .define("x".to_string(), Expr::Number(20.0)); // Shadow

        assert_eq!(inner_env.borrow().get("x"), Some(Expr::Number(20.0)));
        // Ensure outer environment is not affected
        assert_eq!(outer_env.borrow().get("x"), Some(Expr::Number(10.0)));
    }

    #[test]
    fn get_undefined_variable() {
        init_test_logging();
        let env = Environment::new();
        assert_eq!(env.borrow().get("non_existent"), None);
    }

    #[test]
    fn redefine_variable_in_same_env() {
        init_test_logging();
        let env = Environment::new();
        env.borrow_mut().define("x".to_string(), Expr::Number(10.0));
        env.borrow_mut().define("x".to_string(), Expr::Number(20.0)); // Redefine
        assert_eq!(env.borrow().get("x"), Some(Expr::Number(20.0)));
    }
}
