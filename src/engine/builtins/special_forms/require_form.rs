use crate::MODULE_CACHE;
use crate::engine::ast::{Expr, LispModule};
use crate::engine::env::Environment;
use crate::engine::eval::{LispError, eval as main_eval};
use crate::engine::parser;
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use tracing::{debug, error, instrument, trace};

#[instrument(skip(args, _env), fields(args = ?args), ret, err)]
pub fn eval_require(args: &[Expr], _env: Rc<RefCell<Environment>>) -> Result<Expr, LispError> {
    trace!("Executing 'require' special form");
    if args.len() != 1 {
        let msg = format!(
            "'require' expects 1 argument (path string or symbol), got {}",
            args.len()
        );
        error!("{}", msg);
        return Err(LispError::ArityMismatch(msg));
    }

    // The argument to 'require' should be evaluated to get the module name (string or symbol).
    let unevaluated_arg = &args[0];
    let evaluated_arg = main_eval(unevaluated_arg, Rc::clone(&_env))?;

    let module_name_key = match evaluated_arg {
        Expr::String(s) => s.clone(),
        Expr::Symbol(s) => s.clone(),
        _ => {
            let msg = format!(
                "'require' argument must evaluate to a string or symbol, found {:?}",
                evaluated_arg
            );
            error!("{}", msg);
            return Err(LispError::TypeError {
                expected: "String or Symbol path".to_string(),
                found: format!("{:?}", evaluated_arg),
            });
        }
    };

    // Attempt to load from environment (for built-in modules primarily)
    // This allows `(require 'math)` to find the built-in math module.
    if let Some(expr) = _env.borrow().get(&module_name_key) {
        if let Expr::Module(_) = &expr {
            trace!(module_name = %module_name_key, "Found module in environment (likely built-in), returning it.");
            return Ok(expr.clone());
        }
        // If a symbol with the same name exists but is not a module, fall through to filesystem loading.
        // This allows a file like `mymodule.lisp` to be loaded even if a non-module symbol `mymodule` exists.
        trace!(module_name = %module_name_key, value_lisp_str = %expr.to_lisp_string(), "Found symbol in environment but it's not a module, proceeding to filesystem load attempt.");
    }

    // Filesystem loading logic (original logic, now a fallback)
    let mut relative_path_str = module_name_key.clone(); // Use the extracted key for path construction

    // Append .lisp if not already present (original logic)
    if !relative_path_str.ends_with(".lisp") {
        relative_path_str.push_str(".lisp");
    }

    let path_to_check = PathBuf::from(&relative_path_str);
    let absolute_path = if path_to_check.is_absolute() {
        path_to_check
    } else {
        let current_dir = std::env::current_dir().map_err(|e| LispError::ModuleIoError {
            path: path_to_check.clone(), // Use the initial path for error reporting
            kind: e.kind(),
            message: e.to_string(),
        })?;
        current_dir.join(&path_to_check)
    };

    let canonical_path = match fs::canonicalize(&absolute_path) {
        Ok(p) => p,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Err(LispError::ModuleNotFound(absolute_path));
            } else {
                return Err(LispError::ModuleIoError {
                    path: absolute_path,
                    kind: e.kind(),
                    message: e.to_string(),
                });
            }
        }
    };

    // Use module_name_key for logging as evaluated_arg might be partially moved.
    debug!(path_specifier = %module_name_key, resolved_path = %canonical_path.display(), "Path for 'require'");

    {
        let cached_module = MODULE_CACHE.with(|cache_cell| {
            let cache = cache_cell.borrow();
            cache.get(&canonical_path).cloned()
        });
        if let Some(module) = cached_module {
            trace!(path = %canonical_path.display(), "Module found in cache");
            return Ok(module);
        }
    }

    let content = match fs::read_to_string(&canonical_path) {
        Ok(c) => c,
        Err(e) => {
            return Err(LispError::ModuleIoError {
                path: canonical_path,
                kind: e.kind(),
                message: e.to_string(),
            });
        }
    };

    let module_env = Environment::new_with_prelude();
    let mut current_module_input: &str = &content;

    loop {
        current_module_input = current_module_input.trim_start();
        if current_module_input.is_empty() {
            break;
        }
        match parser::parse_expr(current_module_input) {
            Ok((remaining, ast_option)) => {
                if let Some(ast) = ast_option {
                    if let Err(e) = main_eval(&ast, Rc::clone(&module_env)) {
                        error!(module_path = %canonical_path.display(), error = %e, "Error evaluating expression in module");
                        return Err(LispError::ModuleLoadError {
                            path: canonical_path.clone(),
                            source: Box::new(e),
                        });
                    }
                }
                // If ast_option is None, it means a comment or whitespace was parsed, so just continue.
                current_module_input = remaining;
            }
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                if !current_module_input.is_empty() {
                    let parse_err_msg = format!(
                        "Parsing Error in module '{}': {:?}",
                        canonical_path.display(),
                        e
                    );
                    error!("{}", parse_err_msg);
                    return Err(LispError::ModuleLoadError {
                        path: canonical_path.clone(), // Changed here
                        source: Box::new(LispError::Evaluation(format!(
                            "Module parsing error: {}",
                            parse_err_msg
                        ))),
                    });
                }
                break;
            }
            Err(nom::Err::Incomplete(_)) => {
                let msg = format!(
                    "Parsing incomplete in module '{}': More input needed.",
                    canonical_path.display()
                );
                error!("{}", msg);
                return Err(LispError::ModuleLoadError {
                    path: canonical_path.clone(), // Changed here
                    source: Box::new(LispError::Evaluation(msg)),
                });
            }
        }
    }

    let new_module = Expr::Module(LispModule {
        path: canonical_path.clone(),
        env: module_env,
    });

    {
        MODULE_CACHE.with(|cache_cell| {
            let mut cache = cache_cell.borrow_mut();
            cache.insert(canonical_path.clone(), new_module.clone());
        });
        trace!(path = %canonical_path.display(), "Module loaded and cached");
    }

    Ok(new_module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MODULE_CACHE;
    use crate::engine::ast::Expr;
    use crate::engine::env::Environment;
    use crate::engine::eval::LispError; // main_eval is used from parent, eval is for general expr eval
    use crate::logging::init_test_logging;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;
    use std::rc::Rc;
    use tempfile::tempdir; // For creating temporary directories for file-based module tests // To clear cache for specific test scenarios if needed

    // Helper to parse and evaluate a Lisp string containing potentially multiple expressions,
    // returning the result of the last one.
    fn run_require_expr(lisp_code_str: &str, env: Rc<RefCell<Environment>>) -> Result<Expr, LispError> {
        let mut current_input: &str = lisp_code_str;
        let mut last_result: Option<Result<Expr, LispError>> = None;

        loop {
            current_input = current_input.trim_start();
            if current_input.is_empty() {
                break;
            }

            match parser::parse_expr(current_input) {
                Ok((remaining, Some(ast))) => {
                    let eval_result = main_eval(&ast, Rc::clone(&env));
                    if eval_result.is_err() {
                        return eval_result; // Propagate errors immediately
                    }
                    last_result = Some(eval_result);
                    current_input = remaining;
                }
                Ok((remaining, None)) => {
                    // Comment or whitespace, just advance
                    current_input = remaining;
                }
                Err(e) => {
                    // This is a hard parsing error from nom
                    return Err(LispError::Evaluation(format!(
                        "Test parse error for code '{}': {}",
                        lisp_code_str, e
                    )));
                }
            }
        }

        match last_result {
            Some(Ok(expr)) => Ok(expr),
            Some(Err(e)) => Err(e), // Should have been returned earlier, but as a fallback
            None => Err(LispError::Evaluation(
                "No expressions evaluated in test code".to_string(),
            )),
        }
    }

    #[test]
    fn test_require_builtin_math_module_as_symbol() {
        init_test_logging();
        let env = Environment::new_with_prelude(); // Prelude contains built-in modules
        let result = run_require_expr("(require 'math)", Rc::clone(&env));
        match result {
            Ok(Expr::Module(module)) => {
                assert_eq!(module.path, PathBuf::from("builtin:math"));
                // Check if a known math function is in the module's env
                assert!(module.env.borrow().get("+").is_some());
            }
            _ => panic!("Expected LispModule for 'math', got {:?}", result),
        }
    }

    #[test]
    fn test_require_builtin_string_module_as_string_literal() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        let result = run_require_expr("(require \"string\")", Rc::clone(&env));
        match result {
            Ok(Expr::Module(module)) => {
                assert_eq!(module.path, PathBuf::from("builtin:string"));
                assert!(module.env.borrow().get("concat").is_some());
            }
            _ => panic!("Expected LispModule for \"string\", got {:?}", result),
        }
    }

    #[test]
    fn test_require_filesystem_module_simple_name() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        let dir = tempdir().unwrap(); // Create a temp directory

        // Create a dummy module file in the temp directory
        let file_path = dir.path().join("my_fs_module.lisp");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "(let in-module 123)").unwrap();
        drop(file); // Ensure file is closed and written

        // No longer need to change CWD if we use absolute paths in require.
        // let original_dir = std::env::current_dir().unwrap();
        // std::env::set_current_dir(dir.path()).unwrap();

        let canonical_file_path = fs::canonicalize(&file_path).unwrap();
        MODULE_CACHE.with(|mc| mc.borrow_mut().remove(&canonical_file_path));

        // Use the absolute path string in the require call
        let require_expr_str = format!("(require \"{}\")", file_path.to_str().unwrap());
        let result = run_require_expr(&require_expr_str, Rc::clone(&env));

        // std::env::set_current_dir(original_dir).unwrap();

        match result {
            Ok(Expr::Module(module)) => {
                assert_eq!(module.path, canonical_file_path);
                assert_eq!(
                    module.env.borrow().get("in-module"),
                    Some(Expr::Number(123.0))
                );
            }
            _ => panic!("Expected LispModule from filesystem, got {:?}", result),
        }
        // tempdir is cleaned up when `dir` goes out of scope
    }

    #[test]
    fn test_require_filesystem_module_explicit_extension() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("another_module.lisp");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "(let val 789)").unwrap();
        drop(file);

        // let original_dir = std::env::current_dir().unwrap();
        // std::env::set_current_dir(dir.path()).unwrap();

        let canonical_file_path = fs::canonicalize(&file_path).unwrap();
        MODULE_CACHE.with(|mc| mc.borrow_mut().remove(&canonical_file_path));

        // Use the absolute path string in the require call
        // The module name itself (another_module.lisp) will be extracted by eval_require
        let require_expr_str = format!("(require \"{}\")", file_path.to_str().unwrap());
        let result = run_require_expr(&require_expr_str, Rc::clone(&env));

        // std::env::set_current_dir(original_dir).unwrap();

        match result {
            Ok(Expr::Module(module)) => {
                assert_eq!(module.path, canonical_file_path);
                assert_eq!(module.env.borrow().get("val"), Some(Expr::Number(789.0)));
            }
            _ => panic!("Expected LispModule with explicit .lisp, got {:?}", result),
        }
    }

    #[test]
    fn test_require_module_not_found_on_filesystem() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        let dir = tempdir().unwrap();
        // Construct an absolute path to a non-existent file within the temp directory
        let non_existent_path_str = dir.path().join("non_existent_fs_module.lisp");

        let require_expr_str = format!("(require \"{}\")", non_existent_path_str.to_str().unwrap());
        let result = run_require_expr(&require_expr_str, Rc::clone(&env));

        match result {
            Err(LispError::ModuleNotFound(path)) => {
                // The path in the error should be the absolute path we tried to load
                assert_eq!(path, non_existent_path_str);
            }
            _ => panic!("Expected ModuleNotFound, got {:?}", result),
        }
    }

    #[test]
    fn test_require_module_with_runtime_error_in_file() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("runtime_error_module.lisp");
        let mut file = File::create(&file_path).unwrap();
        // This expression will cause a TypeError during evaluation inside the module
        writeln!(file, "(+ 1 \"this-is-not-a-number\")").unwrap();
        drop(file);

        // let original_dir = std::env::current_dir().unwrap();
        // std::env::set_current_dir(dir.path()).unwrap();

        let canonical_file_path = fs::canonicalize(&file_path).unwrap();
        MODULE_CACHE.with(|mc| mc.borrow_mut().remove(&canonical_file_path));

        let require_expr_str = format!("(require \"{}\")", file_path.to_str().unwrap());
        let result = run_require_expr(&require_expr_str, Rc::clone(&env));

        // std::env::set_current_dir(original_dir).unwrap();

        match result {
            Err(LispError::ModuleLoadError { path, source }) => {
                assert_eq!(path, canonical_file_path);
                assert!(matches!(*source, LispError::TypeError { .. }));
            }
            _ => panic!(
                "Expected ModuleLoadError with TypeError source, got {:?}",
                result
            ),
        }
    }

    #[test]
    fn test_require_dynamic_arg_evaluates_to_symbol() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("dyn_mod_sym.lisp");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "(let result-val 777)").unwrap();
        drop(file);

        let canonical_file_path = fs::canonicalize(&file_path).unwrap();
        MODULE_CACHE.with(|mc| mc.borrow_mut().remove(&canonical_file_path));

        // (let mod-name-expr (quote examples/dyn_mod_sym)) ; Note: 'require' will add .lisp
        // (require mod-name-expr)
        // For the test, we need to make sure the symbol path is resolvable from CWD
        // let module_name_for_require = format!("examples/{}", dir.path().file_name().unwrap().to_str().unwrap());
        // let module_file_name_for_let = format!("{}/dyn_mod_sym", module_name_for_require); // This line was unused

        // The `lisp_code` variable was unused, `lisp_code_dynamic` is used below.
        // let lisp_code = format!(
        //     r#"
        //     (let module-path-symbol (quote {}))
        //     (require module-path-symbol)
        //     "#,
        //     module_file_name_for_let // e.g. examples/tempdirname/dyn_mod_sym
        // );
        
        // We need to adjust the path for require to be relative to where cargo test runs (project root)
        // and ensure the temp file is created within an "examples/tempdir" structure if that's how require resolves.
        // Simpler: create the temp file such that its path from CWD is what `require` expects.
        // The `require` form uses `module_name_key` which becomes `examples/tempdir/dyn_mod_sym`.
        // It then appends `.lisp`. So the file needs to be at `examples/tempdir/dyn_mod_sym.lisp`.

        // Let's adjust the test to create the file at a path that `(require 'dyn_mod_sym_test)` would find
        // assuming `require` looks in CWD or CWD/examples.
        // For simplicity, let's assume `require` resolves from CWD.
        
        let temp_module_name = "test_dyn_mod_via_symbol";
        let temp_file_path = dir.path().join(format!("{}.lisp", temp_module_name));
        let mut temp_file = File::create(&temp_file_path).unwrap();
        writeln!(temp_file, "(let dynamic-val 987)").unwrap();
        drop(temp_file);

        let canonical_temp_path = fs::canonicalize(&temp_file_path).unwrap();
        MODULE_CACHE.with(|mc| mc.borrow_mut().remove(&canonical_temp_path));


        let lisp_code_dynamic = format!(
            r#"
            (let mod-name (quote {}))
            (require mod-name)
            "#,
            temp_file_path.to_str().unwrap() // Pass the full path as a symbol/string after quote
        );

        let result = run_require_expr(&lisp_code_dynamic, Rc::clone(&env));

        match result {
            Ok(Expr::Module(module)) => {
                assert_eq!(module.path, canonical_temp_path);
                assert_eq!(
                    module.env.borrow().get("dynamic-val"),
                    Some(Expr::Number(987.0))
                );
            }
            _ => panic!(
                "Expected LispModule from dynamic symbol arg, got {:?}",
                result
            ),
        }
    }

    #[test]
    fn test_require_dynamic_arg_evaluates_to_string() {
        init_test_logging();
        let env = Environment::new_with_prelude();
        let dir = tempdir().unwrap();
        let temp_module_name = "test_dyn_mod_via_string";
        let file_path = dir.path().join(format!("{}.lisp", temp_module_name));
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "(let string-loaded-val 654)").unwrap();
        drop(file);

        let canonical_file_path = fs::canonicalize(&file_path).unwrap();
        MODULE_CACHE.with(|mc| mc.borrow_mut().remove(&canonical_file_path));
        
        let lisp_code = format!(
            r#"
            (let module-path-str "{}")
            (require module-path-str)
            "#,
            file_path.to_str().unwrap() // The string variable holds the full path
        );

        let result = run_require_expr(&lisp_code, Rc::clone(&env));
        match result {
            Ok(Expr::Module(module)) => {
                assert_eq!(module.path, canonical_file_path);
                assert_eq!(
                    module.env.borrow().get("string-loaded-val"),
                    Some(Expr::Number(654.0))
                );
            }
            _ => panic!(
                "Expected LispModule from dynamic string arg, got {:?}",
                result
            ),
        }
    }
}
