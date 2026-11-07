# Using this starter

Notes for you, not the reviewer. The packaging scripts leave this file out of the submission zip.

## What's here

| Path | What it is |
| --- | --- |
| `src/main.rs` | Wiring: arguments, logging, Ctrl-C handling, output, summary |
| `src/cli.rs` | Command-line arguments (clap) |
| `src/pipeline.rs` | Reader, bounded queue, concurrency cap, timeouts, shutdown, with tests |
| `src/task.rs` | The per-item work: the file to rewrite for the real task |
| `README.md` | Submission README template for part 1 |
| `DESIGN.md` | Design template for part 2, with a Mermaid version of the diagram |
| `design/architecture.excalidraw` | Pre-drawn architecture plus spare components to drag in |
| `scripts/package.sh`, `scripts/package.ps1` | Build the submission zip without `target` |
| `samples/items.txt` | Tiny input for a smoke test |

## Tonight

```bash
cargo build && cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo run -- samples/items.txt
```

Building the release profile now means Sunday's release builds take seconds too.

## Sunday, in order

1. Before 7:00: `git init && git add -A && git commit -m "Start from starter template"`.
2. Read both parts of the task. If part 1 is "process many items", keep `pipeline.rs` and rewrite `task.rs`. If it's a different shape (one long-lived stream, a single request, a small server), keep `main.rs` and `cli.rs` and delete `pipeline.rs`.
3. Rename the package in `Cargo.toml` and change the arguments in `cli.rs`.
4. Write the first working version, test it, and commit.
5. Fill in `README.md`. Keep its Notes line saying the scaffolding came from this starter.
6. Part 2: open `design/architecture.excalidraw` at excalidraw.com (Open, or drag the file in), adapt it, and export a PNG to `design/architecture.png`. Fill in `DESIGN.md` and delete the Mermaid block if you don't keep it in sync.
7. `scripts/package.sh` (Windows: `powershell -ExecutionPolicy Bypass -File scripts\package.ps1`) writes `../<folder>-submission.zip` without `target` or this file. Reply to the task email with it.

## Quick changes

| If the task needs | Change |
| --- | --- |
| Results in input order | `buffer_unordered` to `buffered` in `pipeline.rs` |
| Dropping input when the queue is full | `tx.send(...).await` to `tx.try_send(...)` in `read_lines`, counting drops |
| A rate limit | A `tokio::time::interval` ticked in `read_lines` before each send |
| Retries | A loop with backoff (`tokio::time::sleep`) around the work in `task::process` |
| CPU-heavy work per item | `tokio::task::spawn_blocking` inside `task::process` |
| Input that isn't lines | Replace the `lines()` source in `read_lines` |
| HTTP calls | `ctx.http` in `task::process`, then remove the `#[allow(dead_code)]` on `Ctx::http` |
