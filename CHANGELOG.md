# Changelog

All notable changes to this project are documented here. The format loosely follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- [ROADMAP.md](ROADMAP.md) — maintenance posture, P1/P2/P3 backlog, adoption class, and explicit exclusions.
- [CHANGELOG.md](CHANGELOG.md) — this file.
- [SECURITY.md](SECURITY.md) and [CONTRIBUTING.md](CONTRIBUTING.md).
- GitHub Actions CI (`.github/workflows/ci.yml`) running `cargo test`, `cargo test --features=test_runtime`, `cargo test --doc`, `cargo test --examples`, and clippy on stable.
- Runnable `examples/*.rs`: `monadio`, `publisher`, `cor`, `do_notation`, `fp`, `actor`, `actor_ask` (compile-checked and run under `--features=test_runtime`).
- rustdoc coverage across all public modules/items, plus doctests for `fp` and `maybe` APIs.
- Latent-defect regression tests for the async primitives: `WillAsync` and `CountDownLatch` multi-waker and poll-before-start cases, and `BlockingQueue::{take,poll}_result_as_future` (previously untested).
- Runnable doctests for the `foldl!` / `foldr!` macros (completing example coverage across all 11 `fp` macros) and for the opt-in async entry points `MonadIO::to_future` and `Publisher::subscribe_as_stream`.
- README CI status badge (renders live once the workflow runs on the default branch); README now states edition 2021 / MSRV 1.56.
- [docs/PROJECT_NOTES.md](docs/PROJECT_NOTES.md) — project origins (three-era history), cross-language family (`fpGo`/`fpEs`), and a catalog of less-known/"leaked" public APIs (`observe_on`/`subscribe_on` scheduler routing + its no-op trap, `subscribe_blocking_queue` push→pull bridge, `do_m_pattern!`, `map_insert!`, coroutine macros) with scenarios and anti-patterns. README gained a "Less-known / advanced capabilities" section pointing to it.
- Two runnable examples for previously undemonstrated "leaked" capabilities: `examples/scheduler.rs` (`MonadIO::observe_on`/`subscribe_on` thread-hopping, with the `subscribe_on`-alone no-op trap proven by asserting the delivering `ThreadId`) and `examples/publisher_queue.rs` (`Publisher::as_blocking_queue` push→pull bridge). `examples/cor.rs` gained comments documenting the sync/async `yield_from` deadlock invariant.

### Changed

- **Cargo.toml** — `include` now packages `LICENSE` and `README.md`; maintenance badge set to `passively-maintained`.
- **README.md** — removed stale Travis CI badge and custom "docs-online" shield.
- **README.md** — replaced long, non-compiling inline Rust blocks with `cargo run --example <name> --features=test_runtime` commands pointing at maintained example targets.
- **README.md** — documented feature-flag layering (`pure`, `for_futures`, `test_runtime`, module gates, `WillAsync` `Future` requirements).
- **README.md** — documented known behavior: deferred shutdown redesign, `Cor` sync deadlock invariant, and that `thread::sleep` is not synchronization.
- **README.md** — stated that pattern matching is a deferred non-goal (no strikethrough placeholder).
- **clippy.sh** / **publish.sh** — made reproducible on stable; publish now dry-runs, requires an existing release tag, and confirms before publishing.
- Removed legacy `.travis.yml`.
- **Cargo.toml** — adopted **edition 2021** and pinned **MSRV `rust-version = "1.56"`**; removed the dead commented `tokio` dependency and feature line.
- **CI** — added `rustfmt --check` and `rustdoc -D warnings` gates, an MSRV (1.56) job, and a feature-combination build matrix; `clippy.sh` now fails on warnings (`-D warnings`); `publish.sh` refuses a dirty working tree (dropped `--allow-dirty`).
- **src/fp.rs** — reimplemented `compose!`, `pipe!`, and `compose_two` from first principles, removing a CC BY-SA-licensed StackOverflow snippet (semantics unchanged).
- **src/lib.rs** — crate-level docs now list the full 8-module `pure` stack (previously mis-stated as `fp` + `maybe` only).
- Deleted seven vestigial `thread::sleep` synchronization hacks from tests (each was already gated by a latch/flag/waker/queue); mechanical clippy idiom cleanup across `actor`, `common`, `maybe`, `publisher`.

### Fixed

- **`WillAsync` / `CountDownLatch` woke only one awaiter** — the single `Option<Waker>` slot dropped all but the last-registered waker, hanging concurrent awaiters; now stores `Vec<Waker>` and wakes all (draining before waking to stay reentrancy-safe). `WillAsync::start()` now also wakes wakers registered before `start()`, so poll-before-start no longer hangs.
- **`SubscriptionFunc` buffered every value forever (memory leak)** — under `for_futures`, `on_next` unconditionally pushed each delivered value into an internal `cached` stream, but that stream was open from construction (`LinkedListAsync::new()` starts `alive=true`). A callback-only subscription (the common `subscribe_fn` case) has no stream consumer, so the buffer grew without bound. The `cached` stream now starts **closed** and only begins buffering once `as_stream()` opens it — matching the documented open/close intent. Streaming (`subscribe_as_stream`) is unaffected. Regression-guarded by `test_common_subscription_func_no_buffer_until_stream_opened`.
- **`BlockingQueue::stop` doc/dead-code mismatch** — the method and module docs claimed it "drops the sender side" to close the queue, but the code did `let sender = …lock().unwrap(); drop(sender);` which dropped the `MutexGuard`, not the `Sender` (which lives behind `Arc<Mutex<..>>` and cannot be dropped there). Removed the misleading no-op and corrected both docs to state the real mechanism: `stop()` flips the `alive` flag (gating later `offer`/`take`), and a consumer already blocked in `take()` stays blocked until its own `timeout`. No behavior change (verified by `test_sync_blocking_queue_is_alive_and_stop`).

---

## [0.3.5] — 2021-08-21

### Fixed

- Feature sync isolations — `sync` module wiring aligned with feature flags.

### Changed

- Improved `Actor::for_each_child(id, handle)` API for iterating child actors during shutdown-style flows.

---

## [0.3.4] — 2021-08-16

### Changed

- Better APIs for accessing actor children (`get_handle_child`, related helpers).

---

## [0.3.3] — 2021-08-15

### Added

- Actor ask-style patterns (Akka/Erlang inspired) with tests.
- `paralleltest.sh` for repeated concurrent test runs.

### Fixed

- Reduced flaky tests caused by overly short `thread::sleep` timing.

---

## [0.3.2] — 2021-08-14

### Fixed

- Reduced unnecessary `clone()` and `lock()` usage in hot paths.
- `LinkedListAsync` locking improvements.

---

## [0.3.1] — 2021-08-13

### Fixed

- Potential leaked `Future` / `Stream` waker wakeups.
- Publisher `subscribe_blocking_queue` implementation issues.
- `CountDownLatch` future implementation bugs (0.3.0 follow-up).

### Changed

- `Publisher` subscriptions use `Arc<SubscriptionFunc<T>>` instead of `Arc<Mutex<…>>`.
- `LinkedListAsync` extracted and made stream-capable behind `for_futures`.
- Feature tags reorganized for more independent module builds.

---

## [0.3.0] — 2021-08-11

### Added

- **Actor** module — `ActorAsync` with spawn, context state, parent/child handles.
- Expanded tests across actor interfaces.

### Changed

- Major version bump reflecting actor model and feature-flag restructuring.

---

## Earlier 0.2.x — 2021-08 (high level)

- `BlockingQueue` `take_result` / `poll_result` as `Future` (with `for_futures`).
- `CountDownLatch` future support and bug fixes.
- Publisher and MonadIO futures/stream hooks.
- Ongoing reduction of `Arc<Mutex<…>>` usage and subscription rework.

## Earlier 0.1.x — 2018–2021 (high level)

- Initial crate: `MonadIO`, `Publisher`, `Cor` / do-notation macros, `HandlerThread`, `WillAsync`, `fp` macros, `Maybe`.
- Progressive addition of sync primitives, publisher observers, and test coverage.

[Unreleased]: https://github.com/TeaEntityLab/fpRust/compare/v0.3.5...HEAD
[0.3.5]: https://github.com/TeaEntityLab/fpRust/compare/v0.3.4...v0.3.5
[0.3.4]: https://github.com/TeaEntityLab/fpRust/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/TeaEntityLab/fpRust/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/TeaEntityLab/fpRust/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/TeaEntityLab/fpRust/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/TeaEntityLab/fpRust/releases/tag/v0.3.0
