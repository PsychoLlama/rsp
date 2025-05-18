use crate::engine::env::Environment;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use rustyline::history::DefaultHistory; // Corrected import path
use std::cell::RefCell;
use std::rc::Rc;
use tracing::info;

#[tracing::instrument(skip(env))]
pub fn start_repl(env: Rc<RefCell<Environment>>) -> anyhow::Result<()> {
    info!("Starting REPL session with rustyline");
    let mut rl = Editor::<(), DefaultHistory>::new()?; // Specify Helper as () and History as DefaultHistory
    let mut line_number = 1;

    // TODO: Consider loading history if/when implementing it
    // if rl.load_history("history.txt").is_err() {
    //     info!("No previous history found or error loading history.");
    // }

    loop {
        let prompt = format!("lisp ({})> ", line_number);
        let readline = rl.readline(&prompt);

        match readline {
            Ok(line) => {
                // rl.add_history_entry(line.as_str()); // Add to history if implementing
                let trimmed_input = line.trim();

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

    // TODO: Consider saving history if/when implementing it
    // if rl.save_history("history.txt").is_err() {
    //     info!("Error saving history.");
    // }
    Ok(())
}
