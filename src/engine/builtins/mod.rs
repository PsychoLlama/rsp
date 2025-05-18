pub mod log;
pub mod math;
pub mod special_forms;

// LispFunction is not used directly here anymore
// use crate::engine::env::Environment; // No longer needed here
// LispError might be needed if other fns are added
// std::cell::RefCell and std::rc::Rc are no longer needed here
// use std::cell::RefCell;
// use std::rc::Rc;
// use tracing::trace; // trace is not used at this level anymore

// Future built-in functions will go here.

// Native Rust functions callable from Lisp (the "prelude" functions)
// Math functions (native_add, native_equals, native_multiply) are now in the math submodule.
// Log functions are in the log submodule.
// Special form evaluators are in the special_forms submodule.

// native_module_ref function and its tests have been removed as it was unused.

#[cfg(test)]
mod tests {
    // super::native_module_ref is removed
    // use crate::engine::ast::{Expr, LispFunction, LispModule, NativeFunction}; // These are no longer needed here
    // use crate::engine::env::Environment; // No longer needed here
    // use crate::engine::eval::{LispError, eval}; // No longer needed here
    // use crate::logging::init_test_logging; // No longer needed here
    // use std::path::PathBuf; // No longer needed here
    // use std::rc::Rc; // No longer needed here

    // Tests for native functions were here.
    // Tests for native_log_info and native_log_error are in builtins/log/mod.rs
    // Tests for native_module_ref have been removed.
}
