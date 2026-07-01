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
- **Cargo.toml** — `[package.metadata.docs.rs]` with `features = ["for_futures"]` so docs.rs documents the async/Rx surface (`Future`/`Stream` impls, `*_as_future`, `MonadIO::to_future`, `Publisher` stream helpers) instead of only the default `pure` API.
- **docs/PROJECT_NOTES.md** — validated and corrected against git/API ground truth via a parallel multi-lens review (evidence auditor, provenance historian, leaked-API hunter, scenarios/usability, strategic synthesis). Corrected the commit count (204→206); tagged every provenance attribution as **commit-cited** (Cor←Go `28f8178`, `WillAsync`←JavaFuture `24060c2`, `Reduce`←`dtolnay/reduce` `0a53ac6`, Handler/`RawFunc`←Stack Overflow `1f15ba1`/`93a36f8`, GoF Observer←`eliovir/rust-examples` source comment since `9932452`, Akka-style ask `d173df7`) vs **naming-only [INFERENCE]** (Haskell, Java `java.util.concurrent`, Android `Handler` loop, RxJava schedulers); disambiguated the "leaked" homonym (undocumented-feature `b2e17e5` vs resource-leak `6d801ad`/`40ef803`); split the cross-language family into observed parity intent (`maybe.rs:10`) vs inferred sibling APIs; documented five additional easy-to-miss public APIs (`monadio::of`/`From`, `Publisher::map` side-effect, `Cor::yield_from` cross-type, `LinkedListAsync` `Stream`, `RawFunc`/`RawReceiver`); added a standalone `BlockingQueue` scenario (with the `unbounded`/non-interruptible anti-pattern) and corrected the Publisher scenario (stream style is doctest/test-only).

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
- **CI** — the `doc` job now also builds `cargo doc --no-deps` (default `pure`, matching docs.rs) under `-D warnings`, not just `--features=test_runtime`, so broken intra-doc links in the default build fail CI.
- **src/fp.rs** — reimplemented `compose!`, `pipe!`, and `compose_two` from first principles, removing a CC BY-SA-licensed StackOverflow snippet (semantics unchanged).
- **src/lib.rs** — crate-level docs now list the full 8-module `pure` stack (previously mis-stated as `fp` + `maybe` only).
- Deleted seven vestigial `thread::sleep` synchronization hacks from tests (each was already gated by a latch/flag/waker/queue); mechanical clippy idiom cleanup across `actor`, `common`, `maybe`, `publisher`.
- **src/common.rs** — hardened the `shared_thread_pool()` lazy singleton (`for_futures`): replaced `static mut` + `mem::transmute::<Box<_>, *const _>` with an `AtomicPtr` published by a `Release` store inside `Once::call_once` and read by an `Acquire` load. This removes the `Box`→pointer `transmute` and the `static mut` (a hard error under edition 2024), narrowing the `unsafe` block to the single unavoidable leaked-pointer deref. Behavior, public API, and MSRV (1.56) are unchanged (`AtomicPtr`/`Box::into_raw` predate it); guarded by `test_common_shared_thread_pool_concurrent_init_is_singleton`.

### Fixed

- **`WillAsync` / `CountDownLatch` woke only one awaiter** — the single `Option<Waker>` slot dropped all but the last-registered waker, hanging concurrent awaiters; now stores `Vec<Waker>` and wakes all (draining before waking to stay reentrancy-safe). `WillAsync::start()` now also wakes wakers registered before `start()`, so poll-before-start no longer hangs.
- **`SubscriptionFunc` buffered every value forever (memory leak)** — under `for_futures`, `on_next` unconditionally pushed each delivered value into an internal `cached` stream, but that stream was open from construction (`LinkedListAsync::new()` starts `alive=true`). A callback-only subscription (the common `subscribe_fn` case) has no stream consumer, so the buffer grew without bound. The `cached` stream now starts **closed** and only begins buffering once `as_stream()` opens it — matching the documented open/close intent. Streaming (`subscribe_as_stream`) is unaffected. Regression-guarded by `test_common_subscription_func_no_buffer_until_stream_opened`.
- **`BlockingQueue::stop` doc/dead-code mismatch** — the method and module docs claimed it "drops the sender side" to close the queue, but the code did `let sender = …lock().unwrap(); drop(sender);` which dropped the `MutexGuard`, not the `Sender` (which lives behind `Arc<Mutex<..>>` and cannot be dropped there). Removed the misleading no-op and corrected both docs to state the real mechanism: `stop()` flips the `alive` flag (gating later `offer`/`take`), and a consumer already blocked in `take()` stays blocked until its own `timeout`. No behavior change (verified by `test_sync_blocking_queue_is_alive_and_stop`).
- **`HandlerThreadInner::post` accepted work after stop** — `Handler::stop` promised "stop accepting new jobs," but `post()` always enqueued into the inner `BlockingQueue`. A post after stop could leak forever into an un-drained queue or wake a worker blocked in `take()` and execute one more job after stop. `post()` now rejects only the stopped state (`started && !alive`), preserving pre-start queueing. Regression-guarded by `test_handler_inner_post_after_stop_is_rejected` and `test_handler_post_before_start_is_preserved`.
- **`ActorAsync::stop` left mailbox handles accepting sends** — stopping an actor cleared the actor alive flag but left the mailbox `BlockingQueue` alive, so `HandleAsync::send` after stop still queued messages that would never be processed. `stop()` now also stops the mailbox queue so later sends are rejected, while still not interrupting a thread already blocked in `take()` (shutdown redesign remains deferred). Regression-guarded by `test_actor_handle_send_after_stop_is_rejected`.
- **`CountDownLatch::wait` released only one blocking waiter** — `countdown()` used `Condvar::notify_one()`, so when the count reached zero only one waiting thread was guaranteed to wake. It now uses `notify_all()` at zero, matching Java `CountDownLatch` semantics and the existing async multi-waker behavior. Regression-guarded by `test_sync_countdownlatch_releases_all_waiters`.
- **Reentrant waker deadlock risks** — `WillAsync`, `CountDownLatch`, and `LinkedListAsync` now release state/waker locks before invoking `wake()`/`wake_all_wakers()`/`notify_all()`, so synchronous re-polling wakers cannot block on the same non-reentrant mutexes during wakeup. Verified by existing async stream/future tests.
- **`Cor::stop` vestigial no-op** — `stop()` ran `drop(self.op_ch_sender.lock().unwrap());`, which (like the old `BlockingQueue::stop`) dropped the `MutexGuard`, not the shared `Sender`, so it never closed the op channel. Removed the no-op and clarified that cooperative stop is delivered solely by the `alive` flag (a `yield_from` to a stopped `Cor` returns `None` because `receive()` early-returns). Behavior-preserving; also removes one lock held under `started_alive`. The module-level doc still claimed cooperative stop "closes channels"; corrected it to match (the shared op channel is not closed). Verified by `test_cor_*` (pure + `test_runtime`).
- **`Publisher::delete_observer` closed streams for non-members and duplicates too early** — stream closure happened before confirming the subscription belonged to that publisher, so deleting a non-member could close another publisher's stream. With duplicate registrations, removing one occurrence closed the shared stream while another observer still remained. `delete_observer` now closes only after removing a matching observer, and only when no matching observer remains in that publisher. Regression-guarded by `test_publisher_delete_non_member_does_not_close_subscription_stream` and `test_publisher_delete_duplicate_keeps_stream_open_until_last_observer`.
- **Broken rustdoc intra-doc links in the default (`pure`) build** — `LinkedListAsync`'s `Stream` mention, and `sync` module/`WillAsync`/`CountDownLatch`/`BlockingQueue` links to `Future` and `poll_result_as_future`/`take_result_as_future`, referenced `for_futures`-only items and so failed to resolve when that feature was off. Because docs.rs builds default features and the CI `doc` job only checked `--features=test_runtime`, this shipped undetected: `cargo doc` (and docs.rs) rendered eight broken links. Links to always-available items now use a fully-qualified path (`std::future::Future`); links to `for_futures`-only items are plain code spans. `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps` (pure) now passes.
- **README claimed `cor_yield_from!` documents the sync-deadlock invariant, but it didn't** — the README ("Known behavior") stated that `cor_start!`, `set_async`, **and** `cor_yield_from!` document the sync↔sync `yield_from` deadlock in `src/cor.rs`, yet only the first two carried the note. Since `cor_yield_from!` is the operation that actually deadlocks and the macro users call, added the invariant to its rustdoc (pointing at `cor_start!` / `set_async`). The `cor_start!` macro doc-links use the explicit `crate::cor_start` path so they resolve regardless of textual position relative to the macro definition.
- **`BlockingQueue` struct doc contradicted the code (claimed "bounded")** — the rustdoc summary read "Thread-safe **bounded** channel wrapper (Java `BlockingQueue`-like)", but the queue is backed by `mpsc::channel()` (unbounded) and its own `put` note says "there's no maximum size", and `docs/PROJECT_NOTES.md` correctly calls it unbounded. Corrected the summary to state it is **unbounded** and that, unlike a typical bounded Java `BlockingQueue`, `put`/`offer` never block on capacity. Doc-only; verified by `RUSTDOCFLAGS='-D warnings' cargo doc` (pure/for_futures/test_runtime) and the `sync` test suite.

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
