//! The per-item work. This is the file to rewrite for the real task: change
//! `Input`, `Output`, `parse` and `process`, and put any shared state that
//! `process` needs (clients, config, caches) in `Ctx`.

use std::fmt;
use std::time::{Duration, Instant};

use anyhow::Result;
use serde::Serialize;

/// Shared state handed to every `process` call. Keep it cheap to clone:
/// `reqwest::Client` is already a handle to a shared connection pool.
#[derive(Debug, Clone)]
pub struct Ctx {
    /// Not used by the placeholder `process`; there for HTTP tasks.
    #[allow(dead_code)]
    pub http: reqwest::Client,
}

impl Ctx {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;
        Ok(Self { http })
    }
}

/// One unit of work, parsed from one input line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    pub line_no: usize,
    pub text: String,
}

/// The result of processing one `Input`.
#[derive(Debug, Clone, Serialize)]
pub struct Output {
    pub line_no: usize,
    pub chars: usize,
    pub elapsed_ms: u64,
}

impl fmt::Display for Output {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "line {}\t{} chars\t{} ms",
            self.line_no, self.chars, self.elapsed_ms
        )
    }
}

/// Parses one input line. `Ok(None)` skips the line; `Err` counts it as
/// malformed and the run carries on.
pub fn parse(line_no: usize, line: &str) -> Result<Option<Input>> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Ok(None);
    }
    Ok(Some(Input {
        line_no,
        text: line.to_string(),
    }))
}

/// Processes one item. Many of these run at once, so never block in here:
/// use async I/O, and move heavy CPU work to `tokio::task::spawn_blocking`.
pub async fn process(_ctx: Ctx, input: Input) -> Result<Output> {
    let started = Instant::now();
    tokio::time::sleep(Duration::from_millis(20)).await; // placeholder for the real work
    Ok(Output {
        line_no: input.line_no,
        chars: input.text.chars().count(),
        elapsed_ms: started.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_skips_blank_and_comment_lines() {
        assert_eq!(parse(1, "   ").unwrap(), None);
        assert_eq!(parse(2, "# a comment").unwrap(), None);
        assert_eq!(
            parse(3, "  hello  ").unwrap(),
            Some(Input {
                line_no: 3,
                text: "hello".into()
            })
        );
    }

    #[tokio::test]
    async fn process_keeps_the_line_number() {
        let input = Input {
            line_no: 7,
            text: "héllo".into(),
        };
        let output = process(Ctx::new().unwrap(), input).await.unwrap();
        assert_eq!(output.line_no, 7);
        assert_eq!(output.chars, 5);
    }
}
