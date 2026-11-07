//! Reader -> bounded queue -> concurrent workers -> output callback.
//!
//! Backpressure: the queue between the reader and the workers is bounded. When
//! the workers fall behind, the queue fills and the reader's `send().await`
//! waits, so memory stays flat however fast input arrives. `buffer_unordered`
//! caps how many items are in flight at once.
//!
//! Shutdown: cancelling the token stops the reader and stops new items from
//! starting. Items already in flight finish and are reported.

use std::future::Future;
use std::time::Duration;

use anyhow::Result;
use futures::StreamExt;
use tokio::io::{AsyncBufRead, AsyncBufReadExt};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
use tracing::warn;

#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Most items processed at the same time.
    pub concurrency: usize,
    /// Items the reader may queue ahead of the workers.
    pub queue_capacity: usize,
    /// Time limit for one item.
    pub timeout: Duration,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub read: usize,
    pub skipped: usize,
    pub malformed: usize,
    pub ok: usize,
    pub failed: usize,
    pub timed_out: usize,
}

impl Stats {
    /// Items that were queued but never started because of a shutdown.
    pub fn not_started(&self) -> usize {
        let queued = self.read.saturating_sub(self.skipped + self.malformed);
        queued.saturating_sub(self.ok + self.failed + self.timed_out)
    }
}

/// Runs the pipeline until the input ends or `cancel` fires.
///
/// * `parse` turns a line into an item: `Ok(None)` skips it, `Err` counts it
///   as malformed.
/// * `work` processes one item; up to `cfg.concurrency` run at once, each
///   under `cfg.timeout`.
/// * `on_output` receives each successful result as soon as it is ready,
///   which is not necessarily input order.
pub async fn run<R, I, O, P, W, Fut, F>(
    reader: R,
    cfg: Config,
    cancel: CancellationToken,
    parse: P,
    work: W,
    mut on_output: F,
) -> Result<Stats>
where
    R: AsyncBufRead + Unpin + Send + 'static,
    I: Send + 'static,
    P: Fn(usize, &str) -> Result<Option<I>> + Send + 'static,
    W: Fn(I) -> Fut,
    Fut: Future<Output = Result<O>>,
    F: FnMut(O) -> Result<()>,
{
    let (tx, rx) = mpsc::channel(cfg.queue_capacity);
    let reader_task = tokio::spawn(read_lines(reader, parse, tx, cancel.clone()));

    let timeout = cfg.timeout;
    let mut stats = Stats::default();
    {
        let results = ReceiverStream::new(rx)
            .take_until(cancel.cancelled_owned()) // after a shutdown, start nothing new
            .map(|(line_no, item)| {
                let fut = work(item);
                async move { (line_no, tokio::time::timeout(timeout, fut).await) }
            })
            .buffer_unordered(cfg.concurrency);
        let mut results = std::pin::pin!(results);

        while let Some((line_no, result)) = results.next().await {
            match result {
                Ok(Ok(output)) => {
                    stats.ok += 1;
                    on_output(output)?;
                }
                Ok(Err(err)) => {
                    stats.failed += 1;
                    warn!(line_no, "failed: {err:#}");
                }
                Err(_elapsed) => {
                    stats.timed_out += 1;
                    warn!(line_no, "timed out after {timeout:?}");
                }
            }
        }
    } // the stream and its end of the queue drop here, so a waiting reader stops

    let counts = reader_task.await??;
    stats.read = counts.lines;
    stats.skipped = counts.skipped;
    stats.malformed = counts.malformed;
    Ok(stats)
}

#[derive(Debug, Default)]
struct ReadCounts {
    lines: usize,
    skipped: usize,
    malformed: usize,
}

async fn read_lines<R, I, P>(
    reader: R,
    parse: P,
    tx: mpsc::Sender<(usize, I)>,
    cancel: CancellationToken,
) -> Result<ReadCounts>
where
    R: AsyncBufRead + Unpin,
    P: Fn(usize, &str) -> Result<Option<I>>,
{
    let mut lines = reader.lines();
    let mut counts = ReadCounts::default();
    loop {
        let line = tokio::select! {
            biased;
            _ = cancel.cancelled() => break,
            line = lines.next_line() => line?,
        };
        let Some(line) = line else { break }; // end of input
        counts.lines += 1;
        let line_no = counts.lines;
        match parse(line_no, &line) {
            Ok(Some(item)) => {
                // Waits while the queue is full: this is the backpressure.
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => break,
                    sent = tx.send((line_no, item)) => {
                        if sent.is_err() {
                            break; // the workers have stopped
                        }
                    }
                }
            }
            Ok(None) => counts.skipped += 1,
            Err(err) => {
                counts.malformed += 1;
                warn!(line_no, "skipping malformed line: {err:#}");
            }
        }
    }
    Ok(counts)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use anyhow::anyhow;

    use super::*;

    fn cfg(concurrency: usize) -> Config {
        Config {
            concurrency,
            queue_capacity: 2,
            timeout: Duration::from_secs(1),
        }
    }

    fn parse(_line_no: usize, line: &str) -> Result<Option<String>> {
        match line {
            "" => Ok(None),
            "bad" => Err(anyhow!("not a valid item")),
            other => Ok(Some(other.to_string())),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn counts_every_outcome() {
        let mut outputs = Vec::new();
        let stats = run(
            "a\n\nbad\nfail\nslow\nccc\n".as_bytes(),
            cfg(4),
            CancellationToken::new(),
            parse,
            |item: String| async move {
                match item.as_str() {
                    "fail" => Err(anyhow!("boom")),
                    "slow" => {
                        tokio::time::sleep(Duration::from_secs(60)).await;
                        Ok(0)
                    }
                    _ => Ok(item.len()),
                }
            },
            |len| {
                outputs.push(len);
                Ok(())
            },
        )
        .await
        .unwrap();

        outputs.sort_unstable();
        assert_eq!(outputs, vec![1, 3]);
        assert_eq!(
            stats,
            Stats {
                read: 6,
                skipped: 1,
                malformed: 1,
                ok: 2,
                failed: 1,
                timed_out: 1,
            }
        );
        assert_eq!(stats.not_started(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn never_exceeds_the_concurrency_limit() {
        let input: String = (0..20).map(|i| format!("{i}\n")).collect();
        let in_flight = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let stats = run(
            Cursor::new(input.into_bytes()),
            cfg(3),
            CancellationToken::new(),
            parse,
            |item: String| {
                let in_flight = Arc::clone(&in_flight);
                let peak = Arc::clone(&peak);
                async move {
                    let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(now, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                    Ok(item)
                }
            },
            |_| Ok(()),
        )
        .await
        .unwrap();

        assert_eq!(stats.ok, 20);
        assert_eq!(peak.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn starts_nothing_after_cancel() {
        let cancel = CancellationToken::new();
        cancel.cancel();
        let stats = run(
            "a\nb\n".as_bytes(),
            cfg(2),
            cancel,
            parse,
            |item: String| async move { Ok(item) },
            |_| Ok(()),
        )
        .await
        .unwrap();

        assert_eq!(stats.read, 0);
        assert_eq!(stats.ok, 0);
    }
}
