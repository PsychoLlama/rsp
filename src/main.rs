mod cli; // Add cli module
mod logging; // Add logging module
mod engine; // Add engine module

use anyhow::Result;
use clap::Parser; // Import the Parser trait
use tracing::info;

// Import necessary items for parsing and evaluation
use crate::cli::{Cli, Commands}; // Import new CLI structs
use crate::engine::parser::parse_expr;
use crate::engine::eval::eval;
use crate::engine::env::Environment;
use std::fs; // For file reading
use std::rc::Rc; // For Rc::clone on environment
// std::path::PathBuf is used in cli.rs, not directly here unless for type annotation if needed.

#[tracing::instrument]
fn main() -> Result<()> {
    // Initialize tracing subscriber
    crate::logging::init_logging();

    info!("Starting Lisp interpreter");

    let cli_args = Cli::parse(); // Use the new Cli struct from cli.rs
    info!(cli_args = ?cli_args, "Parsed CLI arguments");

    match cli_args.command {
        Commands::Run(run_args) => {
            info!(run_args = ?run_args, "Executing Run command");
            if let Some(expr_str) = run_args.expr {
                info!(expression = %expr_str, "Received expression string for parsing and evaluation");
                match parse_expr(&expr_str) {
                    Ok((remaining_input, ast)) => {
                        if !remaining_input.trim().is_empty() {
                            eprintln!("Error: Unexpected input found after expression: '{}'", remaining_input);
                        } else {
                            info!(parsed_ast = ?ast, "Successfully parsed expression");
                            let root_env = Environment::new_with_prelude();
                            match eval(&ast, root_env) {
                                Ok(result) => {
                                    info!(evaluation_result = ?result, "Evaluation successful");
                                    println!("{:?}", result);
                                }
                                Err(e) => {
                                    info!(evaluation_error = %e, "Evaluation error");
                                    eprintln!("Evaluation Error: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let err_msg = match e {
                            nom::Err::Incomplete(_) => "Parsing incomplete: More input needed.".to_string(),
                            nom::Err::Error(e) | nom::Err::Failure(e) => {
                                format!("Parsing Error: {:?}", e)
                            }
                        };
                        info!(parsing_error = %err_msg, "Parsing failed");
                        eprintln!("{}", err_msg);
                    }
                }
            } else if let Some(file_path) = run_args.file {
                info!(file_path = %file_path.display(), "Received file path for execution");
                match fs::read_to_string(&file_path) {
                    Ok(content) => {
                        let root_env = Environment::new_with_prelude();
                        let mut last_eval_result: Option<crate::engine::ast::Expr> = None;
                        let mut current_input: &str = &content;

                        loop {
                            current_input = current_input.trim_start();
                            if current_input.is_empty() {
                                break; // All content processed
                            }

                            match parse_expr(current_input) {
                                Ok((remaining, ast)) => {
                                    info!(parsed_ast = ?ast, "Successfully parsed expression from file");
                                    match eval(&ast, Rc::clone(&root_env)) {
                                        Ok(result) => {
                                            last_eval_result = Some(result);
                                        }
                                        Err(e) => {
                                            info!(evaluation_error = %e, "Evaluation error from file expression");
                                            eprintln!("Evaluation Error in file '{}': {}", file_path.display(), e);
                                            return Ok(()); // Stop on first evaluation error
                                        }
                                    }
                                    current_input = remaining;
                                }
                                Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                                    if !current_input.is_empty() {
                                         let err_msg = format!("Parsing Error in file '{}': {:?}", file_path.display(), e);
                                         info!(parsing_error = %err_msg, "Parsing failed in file");
                                         eprintln!("{}", err_msg);
                                         return Ok(()); // Stop on first parsing error
                                    }
                                    break;
                                }
                                Err(nom::Err::Incomplete(_)) => {
                                    eprintln!("Parsing incomplete in file '{}': More input needed.", file_path.display());
                                    return Ok(()); // Stop on incomplete parse
                                }
                            }
                        }

                        if let Some(result) = last_eval_result {
                            info!(final_evaluation_result = ?result, "Final evaluation result from file");
                            println!("{:?}", result);
                        } else {
                            info!("No expressions evaluated from file or file was empty.");
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
    }

    info!("Lisp interpreter finished");
    Ok(())
}
