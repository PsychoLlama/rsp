// This module is only compiled when running tests
#![cfg(test)]

use std::sync::Once;

/// Helper to initialize tracing for tests, ensuring it's only done once,
/// and that test output is captured by the test runner.
pub fn setup_tracing() {
    static TRACING_INIT: Once = Once::new();
    TRACING_INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter("trace") // Show all traces for tests
            .with_test_writer() // Capture output for tests
            .try_init()
            .ok(); // Ignore error if already initialized by another test
    });
}
