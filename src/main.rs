mod cli;
mod engine;
mod logging;
mod repl; // Added repl module declaration

use anyhow::Result;
use clap::Parser;
use tracing::info;

use crate::cli::{Cli, Commands};
use crate::engine::ast::Expr; // Added import for Expr
use crate::engine::env::Environment;
use crate::engine::eval::eval;
use crate::engine::parser::parse_expr;
use std::collections::HashMap; // For MODULE_CACHE
use std::fs;
use std::path::PathBuf; // For MODULE_CACHE keys
use std::rc::Rc;
// Mutex and Lazy are not needed for thread_local!
// use std::sync::Mutex;
// use once_cell::sync::Lazy;
use std::cell::RefCell; // For thread_local!

// Global cache for loaded modules, using thread_local for single-threaded context.
// Key: Canonicalized absolute path to the module file.
// Value: The Expr::Module representing the loaded module.
thread_local! {
    pub(crate) static MODULE_CACHE: RefCell<HashMap<PathBuf, crate::engine::ast::Expr>> =
        RefCell::new(HashMap::new());
}

/// Evaluates a sequence of Lisp expressions from a string.
///
/// Args:
///     source_content: The string containing Lisp expressions.
///     env: The environment to evaluate expressions in.
///     source_name: A descriptive name for the source (e.g., "string expression", "file.lisp") for error messages.
///
/// Returns:
///     Ok((Option<Expr>, bool)): The last evaluated expression and a flag indicating if any expressions were evaluated.
///     Err(String): An error message if parsing or evaluation fails.
#[tracing::instrument(skip(source_content, env), fields(source_name = %source_name))]
pub(crate) fn evaluate_source( // Made pub(crate) to be accessible by the repl module
    source_content: &str,
    env: Rc<RefCell<Environment>>,
    source_name: &str,
) -> Result<(Option<Expr>, bool), String> {
    let mut current_input: &str = source_content;
    let mut last_result: Option<Expr> = None;
    let mut expressions_evaluated = false;

    loop {
        current_input = current_input.trim_start();
        if current_input.is_empty() {
            break; // All input processed
        }

        match parse_expr(current_input) {
            Ok((remaining, ast)) => {
                expressions_evaluated = true;
                info!(parsed_ast = ?ast, "Successfully parsed expression from {}", source_name);
                match eval(&ast, Rc::clone(&env)) {
                    Ok(result) => {
                        info!(evaluation_result = ?result, "Evaluation successful in {}", source_name);
                        last_result = Some(result);
                    }
                    Err(e) => {
                        let err_msg = format!("Evaluation Error in {}: {}", source_name, e);
                        info!(evaluation_error = %e, "Evaluation error from {}", source_name);
                        return Err(err_msg); // Stop on first evaluation error
                    }
                }
                current_input = remaining;
            }
            Err(e) => {
                match e {
                    nom::Err::Incomplete(_) => {
                        let err_msg = format!("Parsing incomplete in {}: More input needed.", source_name);
                        info!(parsing_error = %err_msg, input_at_error = %current_input, "Parsing failed in {}", source_name);
                        return Err(err_msg);
                    }
                    nom::Err::Error(inner_e) => {
                        if expressions_evaluated && current_input.trim().is_empty() {
                            break; // Successfully parsed all available expressions
                        }
                        let err_msg = format!("Parsing Error in {}: {:?}", source_name, inner_e);
                        info!(parsing_error = %err_msg, input_at_error = %current_input, "Parsing failed in {}", source_name);
                        return Err(err_msg);
                    }
                    nom::Err::Failure(inner_e) => {
                        let err_msg = format!("Parsing Error in {}: {:?}", source_name, inner_e);
                        info!(parsing_error = %err_msg, input_at_error = %current_input, "Parsing failed critically in {}", source_name);
                        return Err(err_msg);
                    }
                }
            }
        }
    }
    Ok((last_result, expressions_evaluated))
}

#[tracing::instrument]
fn main() -> Result<()> {
    crate::logging::init_logging();

    info!("Starting Lisp interpreter");

    let cli_args = Cli::parse();
    info!(cli_args = ?cli_args, "Parsed CLI arguments");

    match cli_args.command {
        Commands::Run(run_args) => {
            info!(run_args = ?run_args, "Executing Run command");
            if let Some(expr_str) = run_args.expr {
                info!(expression = %expr_str, "Received expression string for parsing and evaluation");
                let root_env = Environment::new_with_prelude();
                match evaluate_source(&expr_str, root_env, "string expression") {
                    Ok((last_result, expressions_evaluated)) => {
                        if let Some(final_result) = last_result {
                            println!("{:?}", final_result);
                        } else if !expressions_evaluated && !expr_str.trim().is_empty() {
                            // This case might be hit if the string was not empty but contained no parsable expressions.
                            // The parser error would have been handled by evaluate_source.
                            // If it was empty to begin with, nothing is printed, which is fine.
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        return Ok(()); // Stop on error
                    }
                }
            } else if let Some(file_path) = run_args.file {
                info!(file_path = %file_path.display(), "Received file path for execution");
                match fs::read_to_string(&file_path) {
                    Ok(content) => {
                        let file_env = Environment::new_with_prelude();
                        let file_path_str = file_path.display().to_string();

                        match evaluate_source(&content, Rc::clone(&file_env), &file_path_str) {
                            Ok((_last_result, expressions_evaluated)) => {
                                // After evaluating all expressions, construct and print the module.
                                let module_expr =
                                    crate::engine::ast::Expr::Module(crate::engine::ast::LispModule {
                                        path: file_path.clone(), // Use the PathBuf directly
                                        env: file_env,
                                    });

                                if !expressions_evaluated && content.trim().is_empty() {
                                    info!(file_path = %file_path_str, "File is empty, resulting in an empty module environment.");
                                } else if !expressions_evaluated {
                                    info!(file_path = %file_path_str, "File contains no valid expressions, resulting in an empty module environment (beyond prelude).");
                                }

                                info!(module = ?module_expr, "Result of file execution is a module");
                                println!("{:?}", module_expr);
                            }
                            Err(e) => {
                                eprintln!("{}", e);
                                return Ok(()); // Stop on error
                            }
                        }
                    }
                    Err(e) => {
                        info!(file_read_error = %e, "Failed to read file");
                        eprintln!("Error reading file '{}': {}", file_path.display(), e);
                    }
                }
            }
            // Clap should ensure that either expr or file is present, so no 'else' needed here.
        }
        Commands::Repl(_repl_args) => {
            info!("Starting REPL mode");
            let repl_env = Environment::new_with_prelude();
            // The start_repl function no longer takes reader/writer arguments
            if let Err(e) = crate::repl::start_repl(repl_env) {
                eprintln!("REPL exited with an error: {}", e);
            }
        }
    }

    info!("Lisp interpreter finished");
    Ok(())
}
