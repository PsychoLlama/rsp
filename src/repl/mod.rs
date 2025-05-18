use crate::engine::env::Environment;
use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;
use tracing::info;

#[tracing::instrument(skip(env))]
pub fn start_repl(env: Rc<RefCell<Environment>>) -> anyhow::Result<()> {
    info!("Starting REPL session");
    let mut line_number = 1;
    loop {
        print!("lisp ({})> ", line_number);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let trimmed_input = input.trim();

        if trimmed_input.is_empty() {
            continue; // Skip empty input
        }

        if trimmed_input == ".exit" || trimmed_input == "(exit)" {
            info!("Exiting REPL session via user command.");
            println!("Exiting.");
            break;
        }

        // Call evaluate_source from main.rs (which needs to be pub(crate))
        match crate::evaluate_source(trimmed_input, Rc::clone(&env), "repl") {
            Ok((Some(result), _)) => {
                println!("{:?}", result);
            }
            Ok((None, true)) => {
                // Input was valid (e.g., a definition) but produced no single printable result,
                // or it was multiple expressions and the last one was like that.
            }
            Ok((None, false)) => {
                // This case implies that evaluate_source determined no actual expressions were processed,
                // e.g., if input was only comments or whitespace (though `trimmed_input.is_empty()` handles some of this).
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }

        line_number += 1;
    }
    Ok(())
}
