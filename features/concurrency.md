# Feature: concurrency

`source_commit: 18a96af` · `last_verified_at: 2026-09-23` · `verification_status: passed`

User goal: Actor model, coroutines (`cor_do_m`), Handler job posting, and
sync primitives (BlockingQueue, CountdownLatch, WillAsync/WillSync).

## Preconditions

- Repo root; `cargo test` works.

## Entry points

| Area | Files | Tests |
| --- | --- | --- |
| Actor | `src/actor.rs` | `actor::test_*` (9) |
| Coroutine | `src/cor.rs` | `cor::test_*` (10) |
| Handler | `src/handler.rs` | `handler::test_*` (8) |
| sync primitives | `src/sync.rs` | `sync::test_*` (17) |

## Drive

```bash
cargo test actor::
cargo test cor::
cargo test handler::
cargo test sync::
```

## Observable outcomes

- Actor: receives in order; spawn child + forward; context accumulates
  state; double-start safe; send after stop rejected; ask returns reply.
- Coroutine: `cor_do_m` sequences yields; start idempotent; lifecycle states
  correct; `yield_from` on a dead target returns None.
- Handler: jobs run FIFO; post before start preserved; post after stop
  rejected; double-start keeps running.
- sync: BlockingQueue FIFO offer/poll + put/take, poll-empty → None,
  take-timeout → None, is_alive/stop; CountdownLatch releases all waiters,
  zero doesn't block, below-zero safe; WillAsync lifecycle + remove callback.

## Failure paths

- Actor messages out of order or send-after-stop accepted → lifecycle broken.
- `cor_do_m` skipping yields or dead-target yield_from hanging → coroutine
  broken.
- Handler FIFO violated or post-after-stop accepted → ordering/guard broken.
- BlockingQueue poll on empty blocking instead of None → non-blocking
  contract broken; CountdownLatch not releasing all waiters → latch broken.

## Evidence

- `cargo test <module> -- --nocapture` output; cross-thread tests
  (`cross_thread_producer`, `countdownlatch_cross_thread`) are the oracle
  for concurrency claims — a single-threaded pass proves nothing there.
