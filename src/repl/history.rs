use crate::repl::highlighter::ReplHelper; // Import the new helper
use rustyline::history::DefaultHistory;
use rustyline::Editor;
use std::fs;
use std::path::PathBuf;
use tracing::{error, info, warn};

const HISTORY_FILE_NAME: &str = "history.txt";

pub(crate) fn get_history_path() -> Option<PathBuf> {
    let crate_name = env!("CARGO_PKG_NAME");
    dirs::data_dir().or_else(dirs::config_dir).map(|mut path| {
        path.push(crate_name);
        path.push(HISTORY_FILE_NAME);
        path
    })
}

pub(crate) fn load_history_from_path(
    rl: &mut Editor<ReplHelper, DefaultHistory>, // Updated Editor type
    history_path: &PathBuf,
) {
    if let Some(parent_dir) = history_path.parent() {
        if !parent_dir.exists() {
            if let Err(e) = fs::create_dir_all(parent_dir) {
                warn!(
                    "Failed to create history directory {}: {}",
                    parent_dir.display(),
                    e
                );
                // If directory creation fails, we probably can't load/save history.
                // The subsequent load_history will likely fail and log it.
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
}

pub(crate) fn save_history_to_path(
    rl: &mut Editor<ReplHelper, DefaultHistory>, // Updated Editor type
    history_path: &PathBuf,
) {
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
