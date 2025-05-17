mod ast;
mod eval;

use anyhow::Result;
use clap::Parser;

use ast::Expr; // To construct example AST nodes
use eval::LispError; // To handle evaluation errors

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
        // For now, we'll just acknowledge the input and run a predefined example.
        println!(
            "Received expression string: \"{}\" (parsing not yet implemented).",
            expr_str
        );
        println!("Running a predefined example evaluation instead:");

        // Example AST: (list 1 (list 2 3)) - this would be (+ 1 2) in a real scenario
        // but our eval function is too simple for arithmetic yet.
        // Let's try something our current eval can handle or error on gracefully.
        let example_ast = Expr::List(vec![
            Expr::Symbol("some_function".to_string()),
            Expr::Number(1.0),
            Expr::Number(2.0),
        ]);

        println!("Predefined AST: {:?}", example_ast);
        match eval::eval(&example_ast) {
            Ok(result) => println!("Evaluation Result: {:?}", result),
            Err(e) => eprintln!("Evaluation Error: {}", e),
        }
    } else {
        // No expression provided via CLI, run a default example or enter REPL (future).
        println!("No expression provided. Running a default example.");

        // Example 1: Evaluate a number
        let ast1 = Expr::Number(42.0);
        println!("\nEvaluating AST: {:?}", ast1);
        match eval::eval(&ast1) {
            Ok(result) => println!("Result: {:?}", result),
            Err(e) => eprintln!("Error: {}", e),
        }

        // Example 2: Evaluate a symbol (expected to fail)
        let ast2 = Expr::Symbol("my_var".to_string());
        println!("\nEvaluating AST: {:?}", ast2);
        match eval::eval(&ast2) {
            Ok(result) => println!("Result: {:?}", result),
            Err(e) => eprintln!("Error: {}", e),
        }

        // Example 3: Evaluate an empty list
        let ast3 = Expr::List(vec![]);
        println!("\nEvaluating AST: {:?}", ast3);
        match eval::eval(&ast3) {
            Ok(result) => println!("Result: {:?}", result),
            Err(e) => eprintln!("Error: {}", e),
        }

        // Example 4: Evaluate a non-empty list (expected to fail with current eval)
        let ast4 = Expr::List(vec![Expr::Symbol("foo".to_string()), Expr::Number(1.0)]);
        println!("\nEvaluating AST: {:?}", ast4);
        match eval::eval(&ast4) {
            Ok(result) => println!("Result: {:?}", result),
            Err(e) => eprintln!("Error: {}", e),
        }
    }

    Ok(())
}
