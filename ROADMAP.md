# fpRust roadmap

This document states how the project is maintained, what we are working toward, and what is explicitly out of scope.

## Posture

fpRust is **actively maintained on a conservative footing**:

- Preserve existing public APIs unless a change fixes a clear bug or is part of an agreed migration.
- Prefer incremental test coverage, accurate docs, and small correctness fixes over large rewrites.
- Keep feature flags modular so synchronous-only consumers stay lightweight.
- Treat async/futures integration as opt-in (`for_futures`, `test_runtime`), not a breaking default.

We optimize for **clarity and teachability** of FP/reactive patterns in Rust, not for competing with production actor runtimes or full async ecosystems.

## Adoption class

| Class | Fit |
|-------|-----|
| **Learning & prototyping** | Good — examples and tests demonstrate monads, pub/sub, coroutines, and actors without heavy frameworks. |
| **Selective library use** | Good — import individual modules (`fp`, `maybe`, `sync`, `publisher`, …) behind feature flags. |
| **Production orchestration** | **Poor fit today** — lifecycle/shutdown semantics are ad hoc; see exclusions below. |
| **Pattern-matching DSL** | **Not supported** — deferred non-goal. |

## Priority backlog

### P1 — correctness, CI, and discoverability

- [x] Replace legacy Travis CI with GitHub Actions (`.github/workflows/ci.yml`).
- [x] Wire a README CI status badge to the live pipeline (badge added; renders live once the workflow runs on the default branch).
- [x] Ship maintained `examples/*.rs` targets referenced from README (`cargo run --example … --features=test_runtime`).
- [x] Document feature-flag matrix in README (kept in sync with `Cargo.toml`).
- [x] Expand Future-polling edge tests (WillAsync/CountDownLatch multi-waker & poll-before-start; BlockingQueue `*_as_future`).

### P2 — API polish and async ergonomics

- [x] Audit remaining `thread::sleep` in tests/examples; replaced with latch/queue/waker-gated synchronization (0 sleeps remain in tests; examples already sleep-free).
- [x] Review `MonadIO::to_future` and `Publisher::subscribe_as_stream` ergonomics under `test_runtime` — signatures kept (posture: preserve public APIs); added runnable usage doctests so both opt-in async entry points are discoverable.
- [x] Clarify `fp` macro docs on docs.rs — all 11 macros now carry runnable doctests (`foldl!`/`foldr!` were the last gap). `Maybe` API already documented with examples.
- [x] Pin MSRV (`rust-version = "1.56"`) and adopt edition 2021.

### P3 — longer horizon

- [x] Contributor guide (coding style, `paralleltest.sh` usage) — see [CONTRIBUTING.md](CONTRIBUTING.md).

The commented tokio dependency has been removed from `Cargo.toml`; integrate with ecosystem runtimes instead of in-crate experiments.

## Explicitly excluded (blocked or non-goals)

### Unified Actor / Handler / Cor shutdown redesign — **excluded until designed**

Today each type implements `stop()` independently:

- **Handler** — stops the worker thread; posted jobs may not drain predictably.
- **Cor** — sets `is_alive` false; pending `yield_from` to a stopped coroutine returns `None`.
- **Actor** — message-driven `stop()` with manual child shutdown in user code.

There is **no shared shutdown protocol** (no guaranteed flush, no structured parent→child teardown order). Redesigning this touches every async primitive and needs a written design + migration plan. **It is not on the P1/P2 execution list** until that design exists.

### Pattern matching — **deferred non-goal**

Haskell/Rust-style pattern-matching macros or syntax extensions are **not targeted**. Do not interpret their absence as a removed v1 feature.

### Grand framework ambitions

fpRust will not become a full actor system, workflow engine, or tokio replacement. Integrate with ecosystem crates instead.

## How to propose changes

1. Open an issue describing the use case and which adoption class it serves.
2. For API changes, note affected feature flags and whether `test_runtime` tests cover the behavior.
3. For shutdown/lifecycle changes, start with a short design doc — implementation PRs without that doc will be deferred.

## Related documents

- [README.md](README.md) — features, examples, known limitations
- [CHANGELOG.md](CHANGELOG.md) — release and unreleased notes
