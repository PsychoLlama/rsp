// Declare modules for each special form
pub mod fn_form;
pub mod if_form;
pub mod let_form;
pub mod quote_form;
pub mod require_form;

// Re-export public evaluation functions
pub use fn_form::eval_fn;
pub use if_form::eval_if;
pub use let_form::eval_let;
pub use quote_form::eval_quote;
pub use require_form::eval_require;
