/// Initializes tracing for general application use.
/// Configures the default log level via the RUST_LOG environment variable
/// (e.g., RUST_LOG=rust_lisp_interpreter=trace,info).
pub fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

/// Initializes tracing specifically for tests.
/// Ensures it's only done once, sets a default trace level,
/// and captures output for the test runner.
#[cfg(test)]
pub fn init_test_logging() {
    // Ensures it's only done once, sets a default trace level,
    // and captures output for the test runner.
    // Using std::sync::Once for this pattern.
    static TRACING_INIT: std::sync::Once = std::sync::Once::new();
    TRACING_INIT.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter("trace") // Show all traces for tests
            .with_test_writer() // Capture output for tests
            .try_init()
            .ok(); // Ignore error if already initialized by another test
    });
}
