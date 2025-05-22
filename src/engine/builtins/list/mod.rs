use crate::engine::ast::Expr;
use crate::engine::env::Environment;
use std::collections::HashMap;
use tracing::trace;

// Placeholder for actual list functions.
// We'll add functions like 'car', 'cdr', 'cons', 'length', 'nth', etc. here.

/// Creates the `list` module with its associated functions.
pub fn create_list_module() -> Expr {
    trace!("Creating list module");
    let list_env_rc = Environment::new(); // Modules have their own environment

    // Scope the mutable borrow so it's dropped before list_env_rc is moved
    {
        let mut _list_env_borrowed = list_env_rc.borrow_mut();
        let _functions_to_define: HashMap<String, Expr> = HashMap::from([
            // Example:
            // (
            //     "length".to_string(),
            //     Expr::NativeFunction(NativeFunction {
            //         name: "list/length".to_string(), // Convention: module_name/function_name
            //         func: native_list_length,
            //     }),
            // ),
        ]);

        // for (name, func_expr) in functions_to_define {
        //     list_env_borrowed.define(name, func_expr);
        // }
    }

    Expr::Module(crate::engine::ast::LispModule {
        // Using a temporary path, or deciding on a convention for "virtual" modules
        path: std::path::PathBuf::from("<builtin_list_module>"),
        env: list_env_rc,
    })
}

// TODO: Add tests for list functions once they are implemented.
// mod tests {
//     use super::*;
//     use crate::engine::eval::{eval, LispError};
//     use crate::engine::parser::parse_expr;
//     use crate::logging::init_test_logging;
//     use std::cell::RefCell;
//     use std::rc::Rc;

//     fn eval_str(code: &str, env: Rc<RefCell<Environment>>) -> Result<Expr, LispError> {
//         let parse_result = parse_expr(code);
//         let (remaining, parsed_expr_option) = match parse_result {
//             Ok((rem, expr_opt)) => (rem, expr_opt),
//             Err(e) => panic!("Test parse error for code '{}': {}", code, e),
//         };

//         if !remaining.is_empty() {
//             panic!(
//                 "Unexpected remaining input after parsing in test for code '{}': {}",
//                 code, remaining
//             );
//         }
//         let parsed_expr = parsed_expr_option.expect("Parsed expression should not be None in test");
//         eval(&parsed_expr, env)
//     }
// }
