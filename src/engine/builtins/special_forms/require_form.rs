use crate::engine::ast::{Expr, LispModule};
use crate::engine::env::Environment;
use crate::engine::eval::{eval as main_eval, LispError};
use crate::engine::parser;
use crate::MODULE_CACHE;
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

    let path_specifier_expr = &args[0];
    let mut relative_path_str = match path_specifier_expr {
        Expr::String(s) => s.clone(),
        Expr::Symbol(s) => s.clone(),
        _ => {
            let msg = format!(
                "'require' argument must be a string or symbol, found {:?}",
                path_specifier_expr
            );
            error!("{}", msg);
            return Err(LispError::TypeError {
                expected: "String or Symbol path".to_string(),
                found: format!("{:?}", path_specifier_expr),
            });
        }
    };

    if !relative_path_str.ends_with(".lisp") {
        relative_path_str.push_str(".lisp");
    }

    let current_dir = std::env::current_dir().map_err(|e| LispError::ModuleIoError {
        path: PathBuf::from(relative_path_str.clone()),
        kind: e.kind(),
        message: e.to_string(),
    })?;
    let mut absolute_path = current_dir;
    absolute_path.push(&relative_path_str);

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

    debug!(path_specifier = ?path_specifier_expr, resolved_path = %canonical_path.display(), "Path for 'require'");

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
            Ok((remaining, ast)) => {
                if let Err(e) = main_eval(&ast, Rc::clone(&module_env)) {
                    error!(module_path = %canonical_path.display(), error = %e, "Error evaluating expression in module");
                    return Err(LispError::ModuleLoadError {
                        path: canonical_path.clone(), // Changed here
                        source: Box::new(e),
                    });
                }
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
    // No tests were present for eval_require in the original mod.rs.
    // If tests are added, they would go here.
    // For example, one might need:
    // use super::eval_require;
    // use crate::engine::ast::Expr;
    // use crate::engine::env::Environment;
    // use crate::engine::eval::{eval, LispError};
    // use crate::logging::init_test_logging;
    // use std::rc::Rc;
    // use std::fs;
    // use std::path::PathBuf;
    // use tempfile::NamedTempFile;
    // use crate::MODULE_CACHE;
}
