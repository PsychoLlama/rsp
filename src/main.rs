mod ast;
mod builtins;
mod env;
mod eval;
mod parser; // Add parser module
mod special_forms;

#[cfg(test)]
mod test_utils;

use anyhow::Result;
use clap::Parser;
use tracing::info;

// Import necessary items for parsing and evaluation
use crate::parser::parse_expr;
use crate::eval::eval;
use crate::env::Environment;
use std::fs; // For file reading
use std::rc::Rc; // For Rc::clone on environment

/// A simple Lisp interpreter written in Rust.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
#[clap(name = "rust-lisp-interpreter", bin_name = "rust-lisp-interpreter")]
struct Cli {
    /// Lisp expression string to evaluate.
    #[clap(short, long, group = "input_source")]
    expression: Option<String>,

    /// Path to a Lisp file to execute.
    #[clap(short, long, group = "input_source")]
    file: Option<String>,
    // input_source group makes --expression and --file mutually exclusive
}

#[tracing::instrument]
fn main() -> Result<()> {
    // Initialize tracing subscriber
    // You can configure the default log level via the RUST_LOG environment variable
    // e.g., RUST_LOG=rust_lisp_interpreter=trace,info cargo run
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Lisp interpreter");

    let cli = Cli::parse();
    info!(?cli, "Parsed CLI arguments");

    if let Some(file_path) = cli.file {
        info!(file_path = %file_path, "Received file path for execution");
        match fs::read_to_string(&file_path) {
            Ok(content) => {
                let root_env = Environment::new_with_prelude();
                let mut last_eval_result: Option<crate::ast::Expr> = None;
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
                                    eprintln!("Evaluation Error in file '{}': {}", file_path, e);
                                    return Ok(()); // Stop on first evaluation error
                                }
                            }
                            current_input = remaining;
                        }
                        Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                            // If it's an error and there was still input, it's a real parse error
                            if !current_input.is_empty() {
                                 let err_msg = format!("Parsing Error in file '{}': {:?}", file_path, e);
                                 info!(parsing_error = %err_msg, "Parsing failed in file");
                                 eprintln!("{}", err_msg);
                                 return Ok(()); // Stop on first parsing error
                            }
                            // If current_input was empty, this error might be due to trying to parse empty string,
                            // which is fine if we're at the end of content. The loop condition handles this.
                            break;
                        }
                        Err(nom::Err::Incomplete(_)) => {
                            eprintln!("Parsing incomplete in file '{}': More input needed.", file_path);
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
                eprintln!("Error reading file '{}': {}", file_path, e);
            }
        }
    } else if let Some(expr_str) = cli.expression {
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
                // nom::Err can be complex. For a simple display:
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
    } else {
        info!("No expression or file provided via CLI");
        println!("No expression or file provided. Use --expression <EXPR>, --file <PATH>, or try --help.");
        println!("Run 'cargo test' to see current evaluation capabilities.");
    }

    info!("Lisp interpreter finished");
    Ok(())
}
