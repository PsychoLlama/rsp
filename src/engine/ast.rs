use crate::engine::env::Environment;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

#[derive(Clone)]
pub struct LispFunction {
    pub params: Vec<String>,
    pub body: Box<Expr>,
    pub closure: Rc<RefCell<Environment>>,
}

impl fmt::Debug for LispFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LispFunction")
            .field("params", &self.params)
            .field("body", &self.body)
            .field("closure", &"<captured_env>") // Avoid printing the whole env
            .finish()
    }
}

// Functions are equal if their parameters and body are structurally equal.
// The captured environment is not considered for this PartialEq.
impl PartialEq for LispFunction {
    fn eq(&self, other: &Self) -> bool {
        self.params == other.params && self.body == other.body
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Symbol(String),
    Number(f64),
    List(Vec<Expr>),
    Function(LispFunction),
    NativeFunction(NativeFunction), // New variant for Rust functions
    Bool(bool),
    Nil,
    Module(LispModule), // New variant for modules
    // Future extensions could include:
    // String(String),
}

#[derive(Clone)]
pub struct LispModule {
    pub path: String,
    pub env: Rc<RefCell<Environment>>,
}

impl fmt::Debug for LispModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LispModule")
            .field("path", &self.path)
            .field("env", &"<module_env>") // Avoid printing the whole env
            .finish()
    }
}

// Modules are equal if their paths are the same.
// This assumes paths are unique identifiers for modules.
impl PartialEq for LispModule {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

/// Type alias for a native Rust function that can be called from Lisp.
/// It takes a Vec of already-evaluated Expr arguments and returns a Result<Expr, LispError>.
pub type NativeFn = fn(Vec<Expr>) -> Result<Expr, crate::engine::eval::LispError>; // Forward declare LispError path

#[derive(Clone)]
pub struct NativeFunction {
    pub name: String, // For debugging and identification
    pub func: NativeFn,
}

impl fmt::Debug for NativeFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeFunction")
            .field("name", &self.name)
            .field("func", &"<native_fn_ptr>") // Avoid printing function pointer details
            .finish()
    }
}

// NativeFunctions are considered equal if their names are the same.
// This assumes that native function names are unique within the system.
impl PartialEq for NativeFunction {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
        // Comparing self.func == other.func (function pointers) is also possible
        // but equality by name is often sufficient if names are guaranteed unique.
    }
}

// Helper functions for constructing AST nodes can be added here later.
// For example:
// pub fn symbol(s: &str) -> Expr { Expr::Symbol(s.to_string()) }
// pub fn number(n: f64) -> Expr { Expr::Number(n) }
// pub fn list(elements: Vec<Expr>) -> Expr { Expr::List(elements) }
