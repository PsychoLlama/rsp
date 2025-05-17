#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Symbol(String),
    Number(f64),
    List(Vec<Expr>),
    // Future extensions could include:
    // Bool(bool),
    // String(String),
}

// Helper functions for constructing AST nodes can be added here later.
// For example:
// pub fn symbol(s: &str) -> Expr { Expr::Symbol(s.to_string()) }
// pub fn number(n: f64) -> Expr { Expr::Number(n) }
// pub fn list(elements: Vec<Expr>) -> Expr { Expr::List(elements) }
