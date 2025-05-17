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
// Rc and RefCell are not directly used here anymore as Environment::new_with_prelude handles it.


/// A simple Lisp interpreter written in Rust.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
#[clap(name = "rust-lisp-interpreter", bin_name = "rust-lisp-interpreter")]
struct Cli {
    /// Lisp expression string to evaluate.
    /// Parsing this string into an AST is not yet implemented.
    #[clap(short, long)]
    expression: Option<String>,
    // Consider adding a flag for REPL mode in the future:
    // #[clap(short, long, action)]
    // repl: bool,
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

    if let Some(expr_str) = cli.expression {
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
                        // e is nom::error::Error<I> or nom::error::VerboseError<I>
                        // For simplicity, just format it. You might want more detailed error reporting.
                        format!("Parsing Error: {:?}", e)
                    }
                };
                info!(parsing_error = %err_msg, "Parsing failed");
                eprintln!("{}", err_msg);
            }
        }
    } else {
        info!("No expression provided via CLI");
        // No expression provided via CLI.
        println!("No expression provided. Use --expression <EXPR> or try --help.");
        println!("Run 'cargo test' to see current evaluation capabilities.");
    }

    info!("Lisp interpreter finished");
    Ok(())
}
