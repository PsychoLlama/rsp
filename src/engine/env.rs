use crate::engine::ast::{Expr, NativeFunction};
use crate::engine::builtins::{native_add, native_equals, native_multiply}; // Added native_multiply
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

        // Add prelude functions
        {
            let mut env_borrowed = env_rc.borrow_mut();
            env_borrowed.define(
                "+".to_string(),
                Expr::NativeFunction(NativeFunction {
                    name: "+".to_string(),
                    func: native_add,
                }),
            );
            env_borrowed.define(
                "=".to_string(),
                Expr::NativeFunction(NativeFunction {
                    name: "=".to_string(),
                    func: native_equals,
                }),
            );
            env_borrowed.define(
                "*".to_string(),
                Expr::NativeFunction(NativeFunction {
                    name: "*".to_string(),
                    func: native_multiply,
                }),
            );
            // Add other prelude functions here as they are implemented
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
