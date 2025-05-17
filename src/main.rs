mod ast;
mod eval;

use anyhow::Result;
use clap::Parser;

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

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(expr_str) = cli.expression {
        // TODO: Implement parsing of `expr_str` into an `ast::Expr`.
        println!(
            "Received expression string: \"{}\". Parsing and evaluation of strings is not yet implemented.",
            expr_str
        );
        println!("Run 'cargo test' to see current evaluation capabilities.");
    } else {
        // No expression provided via CLI.
        println!("No expression provided. Use --expression <EXPR> or try --help.");
        println!("Run 'cargo test' to see current evaluation capabilities.");
    }

    Ok(())
}
