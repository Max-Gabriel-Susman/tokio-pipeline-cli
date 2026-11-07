# <tool name>

<One sentence: what this tool does and for whom.>

## Run it

```bash
cargo run --release -- --help
cargo run --release -- samples/items.txt --concurrency 16 --format json
cat samples/items.txt | cargo run --release
```

Logs and the final summary go to stderr; results go to stdout. Set `RUST_LOG=debug` for more detail.

## How it works

- A reader task parses input line by line and sends items into a bounded queue (`--queue-capacity`). When the workers fall behind, the queue fills and the reader waits, so memory stays flat however fast input arrives.
- Up to `--concurrency` items are processed at once, each with a `--timeout-ms` limit.
- Results are written as soon as each item finishes, so the output is not in input order.
- Malformed lines are skipped and counted; failed and timed-out items are logged and counted. The run carries on either way.
- The first Ctrl-C stops new work and lets in-flight items finish; a second Ctrl-C exits immediately. Exit code 130 means the run was interrupted.

## Design decisions

- <Why this concurrency model, e.g. `buffer_unordered` over spawned tasks>
- <How errors are handled and reported>
- <Anything else a reviewer should know>

## Assumptions

- <Assumption about the input, the environment or the requirements>

## Testing

```bash
cargo test
```

- <What the tests cover>

## With more time

- <Next improvement>

## Notes

The scaffolding (argument parsing, logging, Ctrl-C handling and the reader/worker pipeline) comes from a starter template I prepared before the assessment. <Optional: where you used AI tools.>
