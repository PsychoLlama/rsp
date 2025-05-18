use crate::engine::env::Environment;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;
use std::cell::RefCell;
use std::rc::Rc;
use tracing::{info, warn}; // Removed 'error' as it's handled in history.rs

mod history; // Declare the new history module

#[tracing::instrument(skip(env))]
pub fn start_repl(env: Rc<RefCell<Environment>>) -> anyhow::Result<()> {
    info!("Starting REPL session with rustyline");
    let mut rl = Editor::<(), DefaultHistory>::new()?;
    let mut line_number = 1;

    let history_path_opt = history::get_history_path();

    if let Some(ref history_path) = history_path_opt {
        history::load_history_from_path(&mut rl, history_path);
    } else {
        warn!("Could not determine history file path. History will not be saved or loaded.");
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
        history::save_history_to_path(&mut rl, history_path);
    }
    // The warning about not being able to save is handled when history_path_opt is None earlier.
    Ok(())
}
