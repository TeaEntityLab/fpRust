# fpRust

[![tag](https://img.shields.io/github/tag/TeaEntityLab/fpRust.svg)](https://github.com/TeaEntityLab/fpRust)
[![Crates.io](https://img.shields.io/crates/d/fp_rust.svg)](https://crates.io/crates/fp_rust)
[![docs.rs](https://docs.rs/fp_rust/badge.svg)](https://docs.rs/fp_rust/)
[![CI](https://github.com/TeaEntityLab/fpRust/actions/workflows/ci.yml/badge.svg)](https://github.com/TeaEntityLab/fpRust/actions/workflows/ci.yml)

[![license](https://img.shields.io/github/license/TeaEntityLab/fpRust.svg?style=social&label=License)](https://github.com/TeaEntityLab/fpRust)
[![stars](https://img.shields.io/github/stars/TeaEntityLab/fpRust.svg?style=social&label=Stars)](https://github.com/TeaEntityLab/fpRust)
[![forks](https://img.shields.io/github/forks/TeaEntityLab/fpRust.svg?style=social&label=Fork)](https://github.com/TeaEntityLab/fpRust)

Monad and functional-programming utilities for Rust — Rx-style streams, coroutines, actors, and small FP helpers.

## Why

Functional and reactive patterns are awkward to express in Rust, and few crates cover this niche. fpRust fills part of that gap. See [ROADMAP.md](ROADMAP.md) for maintenance posture and priorities.

**Rust:** edition 2021, MSRV `1.56` (`rust-version` in `Cargo.toml`). The default `pure` build pulls no async dependencies.

## Feature flags

Cargo features are layered. Defaults enable the synchronous `pure` stack only.

| Feature | What it enables |
|---------|-----------------|
| `pure` (default) | `fp`, `maybe`, `sync`, `cor`, `actor`, `handler`, `monadio`, `publisher` modules |
| `for_futures` | Adds optional `futures` / `futures-test` dependencies only — **does not** expose APIs by itself |
| `test_runtime` | `pure` + `for_futures` — use for `cargo test`, examples, and any code that needs `Future` integrations |

Module-scoped flags (`fp`, `maybe`, `sync`, `cor`, `actor`, `handler`, `monadio`, `publisher`) are included via `pure`. APIs such as `MonadIO::to_future()` or `Publisher::subscribe_as_stream()` are gated on **`for_futures` plus the owning module** (for example `monadio` or `publisher`).

`WillAsync` is built when `publisher` and `handler` are enabled. Its `Future` implementation requires **`for_futures` + `publisher` + `handler`** — equivalently, enable `test_runtime` (or `pure` with `for_futures`).

`CountDownLatch`’s `Future` impl needs only `for_futures` (and `sync`, which `pure` already pulls in).

## Capabilities

- **MonadIO** (`fp_rust::monadio`) — map / fmap / subscribe, sync and async scheduling via `HandlerThread`
- **Publisher** (`fp_rust::publisher`) — pub/sub with optional handler-backed delivery
- **FP helpers** (`fp_rust::fp`) — `compose!`, `pipe!`, `map!`, `reduce!`, `filter!`, `foldl!`, `foldr!`, and related macros
- **Sync primitives** (`fp_rust::sync`, `fp_rust::handler`) — `BlockingQueue`, `HandlerThread`, `WillAsync`, `CountDownLatch`
- **Cor** (`fp_rust::cor`) — Pythonic-generator-style coroutines with `yield` / `yield_from` and `do_m!` do-notation macros
- **Actor** (`fp_rust::actor`) — lightweight actor model with `Context`, parent/child messaging, and ask-style patterns
- **Maybe** (`fp_rust::maybe`) — optional/monad helpers

### Less-known / advanced capabilities

Some public APIs are easy to miss. See [docs/PROJECT_NOTES.md](docs/PROJECT_NOTES.md) for the full catalog, origins, and scenarios.

- **Scheduler routing** — `MonadIO::observe_on` / `subscribe_on` (RxJava-style thread-hopping). **Trap:** with no `observe_on`, `subscribe_on` is a silent no-op and the effect runs on the caller. Set `observe_on` first.
- **Push→pull bridge** — `Publisher::as_blocking_queue` / `subscribe_blocking_queue` forward published values into a `BlockingQueue` for deterministic pulling; `Publisher::subscribe_on` delivers off-thread.
- **Sync↔async bridge** — `BlockingQueue::{take,poll}_result_as_future` (`for_futures`) `.await` a blocking queue.
- **Pattern do-notation** — `do_m_pattern!` extends `do_m!` with typed `let`, reassignment, `exec`, and `ret`.
- **Utility macros** — `map_insert!` (bulk `HashMap` fill), `cor_newmutex_and_start!` / `cor_yield!` (coroutine building), `contains!`, `reverse!`.

### Deferred / non-goals

**Pattern matching** macros or DSL support are **not planned** for the current roadmap. They are explicitly deferred, not silently removed features.

## Known behavior and limitations

Read these before relying on stop/shutdown semantics or coroutine scheduling in production code.

### Stop / shutdown redesign — deferred

`Actor`, `Handler`, and `Cor` each expose `stop()` (and related lifecycle flags) with **ad hoc, per-type semantics**. A unified graceful-shutdown design (ordering guarantees, in-flight work draining, parent/child teardown) is **deferred and blocked on design**. Do not assume Akka- or tokio-like shutdown contracts until that work lands. See [ROADMAP.md](ROADMAP.md).

### Cor sync deadlock invariant

`Cor` defaults to **async** scheduling. Two or more coroutines that **yield to each other while both in sync mode** can deadlock waiting on each other. The safe pattern used in tests and examples:

- Keep the **entry** coroutine sync if you need synchronous control flow.
- Keep coroutines that are **yield targets** async (`set_async(true)`).

`cor_start!`, `set_async`, and `cor_yield_from!` document this invariant in `src/cor.rs`.

### `thread::sleep` is not synchronization

Some legacy snippets used fixed `thread::sleep` delays to “wait” for async handlers, actors, or publishers. That is **not** a synchronization primitive — it races under load. Prefer `CountDownLatch`, `BlockingQueue::take` / `take_result`, or `LinkedListAsync` polling with timeouts (as in the `actor_ask` example and current tests) instead of sleeping and hoping.

## Running examples

Runnable examples live under `examples/`. They require the async/futures stack:

```sh
cargo run --example <name> --features=test_runtime
```

| Example | Demonstrates |
|---------|----------------|
| `monadio` | `MonadIO` sync/async subscribe, handler scheduling |
| `publisher` | `Publisher` pub/sub with handlers |
| `cor` | `Cor` yield / yield_from and the sync/async deadlock invariant |
| `do_notation` | `do_m!` do-notation over coroutines |
| `fp` | `compose!`, `pipe!`, `map!` / `reduce!` / `filter!` pipelines |
| `actor` | Actor spawn, parent/child messaging, shutdown messages |
| `actor_ask` | Ask-style replies via `LinkedListAsync` and `BlockingQueue` |
| `scheduler` | `MonadIO` `observe_on` / `subscribe_on` thread-hopping (and the `subscribe_on`-alone no-op trap) |
| `publisher_queue` | `Publisher::as_blocking_queue` push→pull bridge |

To run the full test suite with futures enabled:

```sh
cargo test --features=test_runtime
```

## Documentation

- API reference: [docs.rs/fp_rust](https://docs.rs/fp_rust/)
- Changelog: [CHANGELOG.md](CHANGELOG.md)
- Roadmap and backlog: [ROADMAP.md](ROADMAP.md)
- Project notes (origins, lineage, less-known APIs): [docs/PROJECT_NOTES.md](docs/PROJECT_NOTES.md)

## License

MIT — see [LICENSE](LICENSE).
