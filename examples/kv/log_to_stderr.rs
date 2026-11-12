use std::env;
use std::io::{self, IsTerminal};

use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

pub(crate) fn log_to_stderr() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with_writer(io::stderr)
        .with_ansi(
            io::stderr().is_terminal()
                && env::var_os("NO_COLOR").is_none_or(|value| value.is_empty()),
        )
        .init();
}
