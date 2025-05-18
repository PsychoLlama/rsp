use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// A simple Lisp interpreter written in Rust.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
#[clap(name = "rsp", bin_name = "rsp")]
#[clap(subcommand_required = true, arg_required_else_help = true)] // Ensures a subcommand is given, or help is printed.
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Evaluates a Lisp expression from a string or executes a Lisp file.
    Run(RunArgs),
    /// Starts an interactive Read-Eval-Print Loop (REPL).
    Repl(ReplArgs),
}

#[derive(Args, Debug)]
pub struct ReplArgs {} // Empty for now, can add options later if needed

#[derive(Args, Debug)]
pub struct RunArgs {
    /// Lisp expression string to evaluate.
    #[clap(short, long, value_name = "LISP_CODE", conflicts_with = "file")]
    pub expr: Option<String>,

    /// Path to a Lisp file to execute.
    #[clap(
        value_name = "FILE_PATH",
        conflicts_with = "expr",
        required_unless_present = "expr"
    )]
    pub file: Option<PathBuf>,
}
