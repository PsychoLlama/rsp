use crate::env::Environment;
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
    Bool(bool),
    Nil,
    // Future extensions could include:
    // String(String),
}

// Helper functions for constructing AST nodes can be added here later.
// For example:
// pub fn symbol(s: &str) -> Expr { Expr::Symbol(s.to_string()) }
// pub fn number(n: f64) -> Expr { Expr::Number(n) }
// pub fn list(elements: Vec<Expr>) -> Expr { Expr::List(elements) }
