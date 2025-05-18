use crate::engine::env::Environment;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory; // Corrected import path
use rustyline::Editor;
use std::cell::RefCell;
use std::fs; // For creating directory
use std::path::PathBuf;
use std::rc::Rc;
use tracing::{error, info, warn};

const HISTORY_FILE_SUBDIR: &str = "rsp"; // Crate name
const HISTORY_FILE_NAME: &str = "history.txt";

fn get_history_path() -> Option<PathBuf> {
    dirs::data_dir().or_else(dirs::config_dir).map(|mut path| {
        path.push(HISTORY_FILE_SUBDIR);
        path.push(HISTORY_FILE_NAME);
        path
    })
}

#[tracing::instrument(skip(env))]
pub fn start_repl(env: Rc<RefCell<Environment>>) -> anyhow::Result<()> {
    info!("Starting REPL session with rustyline");
    let mut rl = Editor::<(), DefaultHistory>::new()?;
    let mut line_number = 1;

    let history_path_opt = get_history_path();

    if let Some(ref history_path) = history_path_opt {
        if let Some(parent_dir) = history_path.parent() {
            if !parent_dir.exists() {
                if let Err(e) = fs::create_dir_all(parent_dir) {
                    warn!(
                        "Failed to create history directory {}: {}",
                        parent_dir.display(),
                        e
                    );
                }
            }
        }
        if history_path.exists() {
            if let Err(err) = rl.load_history(history_path) {
                warn!(
                    "Could not load history from {}: {}",
                    history_path.display(),
                    err
                );
            } else {
                info!("Loaded history from {}", history_path.display());
            }
        } else {
            info!(
                "History file {} does not exist. Will create on exit.",
                history_path.display()
            );
        }
    } else {
        warn!("Could not determine history file path. History will not be saved.");
    }

    loop {
        let prompt = format!("lisp ({})> ", line_number);
        let readline = rl.readline(&prompt);

        match readline {
            Ok(line) => {
                let trimmed_input = line.trim();

                if !trimmed_input.is_empty() {
                    // Add to history only if it's not an empty line
                    if let Err(err) = rl.add_history_entry(line.as_str()) {
                        warn!("Failed to add line to history: {}", err);
                    }
                }

                if trimmed_input.is_empty() {
                    line_number += 1;
                    continue;
                }

                if trimmed_input == ".exit" || trimmed_input == "(exit)" {
                    info!("Exiting REPL session via user command.");
                    println!("Exiting.");
                    break;
                }

                match crate::evaluate_source(trimmed_input, Rc::clone(&env), "repl") {
                    Ok((Some(result), _)) => {
                        println!("{:?}", result);
                    }
                    Ok((None, true)) => {
                        // Valid input, no printable result (e.g., define)
                    }
                    Ok((None, false)) => {
                        // No actual expressions processed (e.g., comments)
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                info!("REPL interrupted (Ctrl-C).");
                println!("Interrupted. Type .exit, (exit), or Ctrl-D to exit.");
                // Optionally, break here or allow continuation.
                // For now, we allow continuation but increment line number.
            }
            Err(ReadlineError::Eof) => {
                info!("REPL EOF detected (Ctrl-D).");
                println!("Exiting.");
                break;
            }
            Err(err) => {
                eprintln!("REPL Readline Error: {:?}", err);
                break;
            }
        }
        line_number += 1;
    }

    if let Some(ref history_path) = history_path_opt {
        if let Err(err) = rl.save_history(history_path) {
            error!(
                "Could not save history to {}: {}",
                history_path.display(),
                err
            );
        } else {
            info!("Saved history to {}", history_path.display());
        }
    }
    Ok(())
}
