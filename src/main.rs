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
            Ok((remaining, ast_option)) => {
                if let Some(ast) = ast_option {
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
                } else {
                    // No actual expression was parsed (e.g., only comments or whitespace).
                    // If remaining is the same as current_input, and current_input is not empty,
                    // it implies that space_or_comment0 consumed nothing, which might be an issue
                    // if current_input was *only* a comment that parse_expr should have consumed entirely.
                    // However, with `preceded(space_or_comment0, opt(terminated(expr_recursive_impl, space_or_comment0)))`,
                    // `remaining` should be the input *after* the initial `space_or_comment0`.
                    // If `ast_option` is None, it means the `opt(...)` part returned None.
                    // This is the correct behavior for comment-only or empty lines after initial whitespace.
                    if remaining.is_empty() && current_input.trim().is_empty() && !expressions_evaluated {
                        // Input was effectively empty (or only comments/whitespace) from the start.
                        // No actual error, just nothing to do.
                    }
                    // If remaining is not empty but ast_option is None, it means the rest of the input
                    // after initial whitespace/comments did not form a valid expression.
                    // This is treated as a parsing error by the logic below if not all input is consumed.
                }
                current_input = remaining;
            }
            Err(e) => { // This is a hard parsing error from nom
                match e {
                    nom::Err::Incomplete(_) => {
                        let err_msg = format!("Parsing incomplete in {}: More input needed.", source_name);
                        info!(parsing_error = %err_msg, input_at_error = %current_input, "Parsing failed in {}", source_name);
                        return Err(err_msg);
                    }
                    nom::Err::Error(ref inner_e) => { // Use ref inner_e to avoid moving
                        // If we have already parsed some expressions and the rest is empty or whitespace,
                        // it's not an error. This check is tricky with the new Option<Expr>.
                        // The `opt` in parse_expr should handle cases where the remaining input is just whitespace.
                        // An error here means `expr_recursive_impl` failed on non-empty, non-comment input.
                        if expressions_evaluated && current_input.trim().is_empty() {
                             // This case might be less relevant now as parse_expr(whitespace) -> Ok(("", None))
                            break; 
                        }
                        let err_msg = format!("Parsing Error in {}: {:?}", source_name, inner_e);
                        info!(parsing_error = %err_msg, input_at_error = %current_input, "Parsing failed in {}", source_name);
                        return Err(err_msg);
                    }
                    nom::Err::Failure(ref inner_e) => { // Use ref inner_e
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
