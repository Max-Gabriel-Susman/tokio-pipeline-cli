use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Args {
    /// Input file with one item per line. Reads stdin when omitted.
    pub input: Option<PathBuf>,

    /// Most items processed at the same time.
    #[arg(short, long, default_value_t = 8)]
    pub concurrency: usize,

    /// Items the reader may queue ahead of the workers. When the queue is
    /// full, the reader waits (backpressure).
    #[arg(long, default_value_t = 1024)]
    pub queue_capacity: usize,

    /// Time limit for one item, in milliseconds.
    #[arg(long, default_value_t = 5_000)]
    pub timeout_ms: u64,

    /// Output format for results on stdout.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    pub format: Format,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    Text,
    Json,
}
