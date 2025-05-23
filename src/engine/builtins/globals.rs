use crate::engine::ast::{Expr, NativeFunction};
use crate::engine::builtins::log::create_log_module;
use crate::engine::builtins::math::{
    create_math_module, native_add, native_divide, native_equals, native_greater_than,
    native_greater_than_or_equal, native_less_than, native_less_than_or_equal, native_multiply,
    native_subtract,
};
use crate::engine::builtins::list::create_list_module;
use crate::engine::builtins::string::create_string_module;
use crate::engine::env::Environment;
use std::cell::RefCell;
use std::rc::Rc;

/// Populates the given environment with global built-in modules and functions.
pub fn populate_globals(env: Rc<RefCell<Environment>>) {
    // Create the math module using its dedicated function
    let math_module = create_math_module();

    // Create the log module using its dedicated function
    let log_module = create_log_module();

    // Create the string module using its dedicated function
    let string_module = create_string_module();

    // Create the list module using its dedicated function
    let list_module = create_list_module();

    // Define functions and modules in the root prelude
    let mut root_env_borrowed = env.borrow_mut();
    root_env_borrowed.define("math".to_string(), math_module);
    root_env_borrowed.define("log".to_string(), log_module);
    root_env_borrowed.define("string".to_string(), string_module);
    root_env_borrowed.define("list".to_string(), list_module);

    // Define shorthand math functions directly in root prelude
    root_env_borrowed.define(
        "+".to_string(),
        Expr::NativeFunction(NativeFunction {
            name: "+".to_string(),
            func: native_add,
        }),
    );
    root_env_borrowed.define(
        "=".to_string(),
        Expr::NativeFunction(NativeFunction {
            name: "=".to_string(),
            func: native_equals,
        }),
    );
    root_env_borrowed.define(
        "*".to_string(),
        Expr::NativeFunction(NativeFunction {
            name: "*".to_string(),
            func: native_multiply,
        }),
    );
    root_env_borrowed.define(
        "-".to_string(),
        Expr::NativeFunction(NativeFunction {
            name: "-".to_string(),
            func: native_subtract,
        }),
    );
    root_env_borrowed.define(
        "/".to_string(),
        Expr::NativeFunction(NativeFunction {
            name: "/".to_string(),
            func: native_divide,
        }),
    );
    root_env_borrowed.define(
        "<".to_string(),
        Expr::NativeFunction(NativeFunction {
            name: "<".to_string(),
            func: native_less_than,
        }),
    );
    root_env_borrowed.define(
        ">".to_string(),
        Expr::NativeFunction(NativeFunction {
            name: ">".to_string(),
            func: native_greater_than,
        }),
    );
    root_env_borrowed.define(
        "<=".to_string(),
        Expr::NativeFunction(NativeFunction {
            name: "<=".to_string(),
            func: native_less_than_or_equal,
        }),
    );
    root_env_borrowed.define(
        ">=".to_string(),
        Expr::NativeFunction(NativeFunction {
            name: ">=".to_string(),
            func: native_greater_than_or_equal,
        }),
    );
}
