use anyhow::Result;
use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{Registry, fmt, prelude::*};

pub fn init(log_dir: &Path) -> Result<WorkerGuard> {
    let file_appender = tracing_appender::rolling::daily(log_dir, "vicuna.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer().with_ansi(false).with_writer(non_blocking);

    Registry::default().with(file_layer).init();

    Ok(guard)
}
