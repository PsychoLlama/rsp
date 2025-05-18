mod cli;
mod engine;
mod logging;

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
                let mut current_input: &str = &expr_str;
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
                            info!(parsed_ast = ?ast, "Successfully parsed expression from string");
                            match eval(&ast, Rc::clone(&root_env)) {
                                Ok(result) => {
                                    info!(evaluation_result = ?result, "Evaluation successful");
                                    last_result = Some(result);
                                }
                                Err(e) => {
                                    info!(evaluation_error = %e, "Evaluation error from string expression");
                                    eprintln!("Evaluation Error: {}", e);
                                    last_result = None; // Clear last result on error
                                    return Ok(()); // Stop on first evaluation error
                                }
                            }
                            current_input = remaining;
                        }
                        Err(e) => {
                            let err_msg = match e {
                                nom::Err::Incomplete(_) => {
                                    "Parsing incomplete: More input needed.".to_string()
                                }
                                nom::Err::Error(inner_e) | nom::Err::Failure(inner_e) => {
                                    // Check if the error occurred on an empty or whitespace-only remaining string.
                                    // If so, it might not be a "real" error but just the end of input.
                                    if current_input.trim().is_empty() && !expressions_evaluated {
                                        // No valid expressions were found at all.
                                        "Error: No valid expressions found in input string.".to_string()
                                    } else if current_input.trim().is_empty() && expressions_evaluated {
                                        // Successfully parsed some, now at the end. Not an error.
                                        break; 
                                    } else {
                                        format!("Parsing Error: {:?}", inner_e)
                                    }
                                }
                            };
                            // Only print error if it's not just the end of input after successful parses.
                            if !(current_input.trim().is_empty() && expressions_evaluated && matches!(e, nom::Err::Error(_))) {
                                info!(parsing_error = %err_msg, "Parsing failed in string expression");
                                eprintln!("{}", err_msg);
                                last_result = None; // Clear last result on error
                            }
                            return Ok(()); // Stop on parsing error or end of input
                        }
                    }
                }

                if let Some(final_result) = last_result {
                    println!("{:?}", final_result);
                } else if !expressions_evaluated && !expr_str.trim().is_empty() {
                    // This case might be hit if the string was not empty but contained no parsable expressions.
                    // The parser error would have been handled above.
                    // If it was empty to begin with, nothing is printed, which is fine.
                }


            } else if let Some(file_path) = run_args.file {
                info!(file_path = %file_path.display(), "Received file path for execution");
                match fs::read_to_string(&file_path) {
                    Ok(content) => {
                        // Create a dedicated environment for this file.
                        let file_env = Environment::new_with_prelude();
                        let mut current_input: &str = &content;
                        let mut expressions_evaluated = false;

                        loop {
                            current_input = current_input.trim_start();
                            if current_input.is_empty() {
                                break; // All content processed
                            }

                            match parse_expr(current_input) {
                                Ok((remaining, ast)) => {
                                    expressions_evaluated = true;
                                    info!(parsed_ast = ?ast, "Successfully parsed expression from file");
                                    // Evaluate in the file's dedicated environment.
                                    // We don't need to store the result of each eval,
                                    // as the side effect is on file_env.
                                    if let Err(e) = eval(&ast, Rc::clone(&file_env)) {
                                        info!(evaluation_error = %e, "Evaluation error from file expression");
                                        eprintln!(
                                            "Evaluation Error in file '{}': {}",
                                            file_path.display(),
                                            e
                                        );

                                        return Ok(()); // Stop on first evaluation error
                                    }
                                    current_input = remaining; // Correctly placed inside the Ok arm
                                }
                                Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                                    if !current_input.is_empty() {
                                        let err_msg = format!(
                                            "Parsing Error in file '{}': {:?}",
                                            file_path.display(),
                                            e
                                        );
                                        info!(parsing_error = %err_msg, "Parsing failed in file");
                                        eprintln!("{}", err_msg);
                                        return Ok(()); // Stop on first parsing error
                                    }
                                    break; // End of parsable content or legitimate error on empty remaining string
                                }
                                Err(nom::Err::Incomplete(_)) => {
                                    eprintln!(
                                        "Parsing incomplete in file '{}': More input needed.",
                                        file_path.display()
                                    );
                                    return Ok(()); // Stop on incomplete parse
                                }
                            }
                        }

                        // After evaluating all expressions, construct and print the module.
                        let module_path_str = file_path.display().to_string();
                        let module_expr =
                            crate::engine::ast::Expr::Module(crate::engine::ast::LispModule {
                                path: file_path.clone(), // Use the PathBuf directly
                                env: file_env,
                            });

                        if !expressions_evaluated && content.trim().is_empty() {
                            info!(file_path = %module_path_str, "File is empty, resulting in an empty module environment.");
                        } else if !expressions_evaluated {
                            info!(file_path = %module_path_str, "File contains no valid expressions, resulting in an empty module environment (beyond prelude).");
                        }

                        info!(module = ?module_expr, "Result of file execution is a module");
                        println!("{:?}", module_expr);
                    }
                    Err(e) => {
                        info!(file_read_error = %e, "Failed to read file");
                        eprintln!("Error reading file '{}': {}", file_path.display(), e);
                    }
                }
            }
            // Clap should ensure that either expr or file is present, so no 'else' needed here.
        }
    }

    info!("Lisp interpreter finished");
    Ok(())
}
