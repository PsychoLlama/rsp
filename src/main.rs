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
// To construct example AST nodes (ast::Expr) and handle evaluation errors (eval::LispError)
// are no longer directly used in main after moving examples to tests.
// The modules themselves are still declared below.

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
        // TODO: Implement parsing of `expr_str` into an `ast::Expr`.
        info!(expression = %expr_str, "Received expression string");
        println!(
            "Received expression string: \"{}\". Parsing and evaluation of strings is not yet implemented.",
            expr_str
        );
        println!("Run 'cargo test' to see current evaluation capabilities.");
    } else {
        info!("No expression provided via CLI");
        // No expression provided via CLI.
        println!("No expression provided. Use --expression <EXPR> or try --help.");
        println!("Run 'cargo test' to see current evaluation capabilities.");
    }

    info!("Lisp interpreter finished");
    Ok(())
}
