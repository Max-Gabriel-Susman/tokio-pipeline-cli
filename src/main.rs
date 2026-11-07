//! Async CLI starter: reads one item per line, processes items concurrently
//! with backpressure and per-item timeouts, writes one result per line, and
//! shuts down cleanly on Ctrl-C. The task-specific code lives in `task.rs`.

mod cli;
mod pipeline;
mod task;

use std::io::{self, BufWriter, Write};
use std::time::Duration;

use anyhow::{ensure, Context, Result};
use clap::Parser;
use tokio::io::{AsyncBufRead, BufReader};
use tokio_util::sync::CancellationToken;
use tracing::warn;
use tracing_subscriber::EnvFilter;

use cli::{Args, Format};
use pipeline::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(io::stderr)
        .init();
    ensure!(args.concurrency > 0, "--concurrency must be at least 1");
    ensure!(
        args.queue_capacity > 0,
        "--queue-capacity must be at least 1"
    );

    let cancel = CancellationToken::new();
    handle_ctrl_c(cancel.clone());

    let input: Box<dyn AsyncBufRead + Unpin + Send> = match &args.input {
        Some(path) => Box::new(BufReader::new(
            tokio::fs::File::open(path)
                .await
                .with_context(|| format!("opening {}", path.display()))?,
        )),
        None => Box::new(BufReader::new(tokio::io::stdin())),
    };
    let cfg = Config {
        concurrency: args.concurrency,
        queue_capacity: args.queue_capacity,
        timeout: Duration::from_millis(args.timeout_ms),
    };
    let ctx = task::Ctx::new()?;
    let mut out = BufWriter::new(io::stdout());

    let stats = pipeline::run(
        input,
        cfg,
        cancel.clone(),
        task::parse,
        move |item| task::process(ctx.clone(), item),
        |output| {
            match args.format {
                Format::Text => writeln!(out, "{output}")?,
                Format::Json => writeln!(out, "{}", serde_json::to_string(&output)?)?,
            }
            Ok(())
        },
    )
    .await?;
    out.flush()?;

    eprintln!(
        "read {} | skipped {} | malformed {} | ok {} | failed {} | timed out {} | not started {}",
        stats.read,
        stats.skipped,
        stats.malformed,
        stats.ok,
        stats.failed,
        stats.timed_out,
        stats.not_started(),
    );
    if cancel.is_cancelled() {
        std::process::exit(130);
    }
    Ok(())
}

/// First Ctrl-C: stop taking new work and let in-flight items finish.
/// Second Ctrl-C: exit immediately.
fn handle_ctrl_c(cancel: CancellationToken) {
    tokio::spawn(async move {
        while tokio::signal::ctrl_c().await.is_ok() {
            if cancel.is_cancelled() {
                eprintln!("second Ctrl-C: exiting now");
                std::process::exit(130);
            }
            warn!("Ctrl-C: finishing in-flight work; press Ctrl-C again to quit now");
            cancel.cancel();
        }
    });
}
