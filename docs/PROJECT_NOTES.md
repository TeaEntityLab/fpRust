# fpRust — Project Notes: Origins, Identity & Less-Known Capabilities

> Durable engineering notes derived from a git-history + API-surface investigation
> (204 commits, 2018‑07 → 2026‑07). This document records *what kind of project this
> is*, *why it is shaped the way it is*, and the *public capabilities that are easy to
> miss*. It is context for maintainers and advanced users; the [README](../README.md)
> remains the primary user guide and [ROADMAP](../ROADMAP.md) the maintenance posture.

## Identity

fpRust is an **educational, polyglot functional-programming / reactive pattern
library** — the Rust member of a tri-language family alongside **`fpGo`** (Go) and
**`fpEs`** (ECMAScript). Its APIs deliberately mirror those siblings (`src/maybe.rs`
documents the `map`/`chain`/`ap` parity), so the same FP/reactive concepts read the
same way across Go, JavaScript, and Rust.

It is **not** a production async runtime or a Tokio/Actix competitor. It optimizes for
**clarity and teachability** of cross-language patterns, and is a good fit for
*learning & prototyping* and *selective module use* — not *production orchestration*
(no unified shutdown protocol; see [ROADMAP](../ROADMAP.md) exclusions).

## Origin story (three eras)

| Era | Window | What landed | Influences |
|-----|--------|-------------|------------|
| **1 — Sync FP + threading core** | 2018‑07 (142 commits) | `Maybe` → `MonadIO` → `Handler`/`HandlerThread` → `BlockingQueue` → `Publisher`/`SubscriptionFunc` → `CountDownLatch` → `Cor` → fp macros (`map!`/`reduce!`/`filter!`/`fold*!`/`compose!`/`pipe!`) → `do_m!` | Haskell (monads, do‑notation), Android (`Handler` loop), Java `java.util.concurrent`, RxJava schedulers, Go coroutines (Cor "from golang version"), GoF Observer (via `eliovir/rust-examples`), `dtolnay/reduce` |
| **2 — Java Future** | 2018‑08 (3 commits) | `WillAsync` ("to achieve JavaFuture") | Java `Future` callback/wait semantics |
| **3 — Async Rust + Actors** | 2021‑06→08 (56 commits) | Rust‑2021 modernization (only external PR ever, #1 `bogdad/fix_compilation`), `futures::Stream` + `Future` integration, `subscribe_as_stream`, real async `CountDownLatch`, `BlockingQueue::{take,poll}_result_as_future`, **Akka‑style `ActorAsync`** (ask/tell + children), `LinkedListAsync`, feature‑flag modularization |

(2026‑07 added test coverage, docs/roadmap, and async‑waker correctness fixes — see [CHANGELOG](../CHANGELOG.md).)

### On the word "leaked"
The 2018‑07‑11 commit **"Add leaked observe/subscribe_on"** is the author's own term
for **capabilities shipped ahead of their documentation** — public, working, but never
surfaced to users. This document is largely about paying down that gap.

## Less-known / "leaked" capabilities

These are fully public and working but absent from the README's main capability list.
Ranked by user value; every claim traced to source.

### 1. `MonadIO::observe_on` / `subscribe_on` — scheduler routing (with a trap)
`src/monadio.rs:105,110`. Inspired by RxJava's `observeOn`/`subscribeOn`, but the
semantics **differ** — verified at `src/monadio.rs:141‑163`:

- `ob_handler == None` → the effect runs **synchronously on the calling thread** and
  **`subscribe_on` is ignored entirely**. So `subscribe_on` *alone* is a silent no‑op.
- `ob_handler == Some` → the effect is posted to the observe handler, which then
  forwards effect execution **and** the subscriber's `on_next` to the subscribe
  handler thread (`Some`) or runs them on the observe thread (`None`).

**Rule:** if you want thread‑hopping, always set `observe_on` first; `subscribe_on`
only takes effect when `observe_on` is set.

### 2. `Publisher::subscribe_blocking_queue` / `as_blocking_queue` — push→pull bridge
`src/publisher.rs:99,107`. Turns the push‑based pub/sub stream into a **pull‑based
`BlockingQueue`**: published values are forwarded into a queue a consumer thread drains
on its own schedule (`queue.take()` / `take_result_as_future().await`). Ideal when the
consumer wants to pull deterministically instead of running in a callback. Note: it is
a delivery bridge, not a bounded backpressure mechanism — see the queue's own limits.

### 3. `Publisher::subscribe_on` — off-thread delivery
`src/publisher.rs:58`. Unlike MonadIO's, this one behaves straightforwardly: with a
handler set, `notify_observers` posts each delivery to that handler thread
(`src/publisher.rs` notify path); with `None`, delivery is synchronous on the publish
caller. Safe to use on its own.

### 4. `BlockingQueue::{take,poll}_result_as_future` — sync↔async bridge
`src/sync.rs`. `for_futures` only. Offloads the blocking `recv` onto the shared futures
thread pool so a synchronous `BlockingQueue` can be `.await`ed from async code.

### 5. `do_m_pattern!` — pattern-matching do-notation
`src/cor.rs:51`. A richer `do_m!`: supports typed `let`, `let mut`, reassignment,
`exec` side-effect statements, `yield_from` of coroutines, and a final `ret`. Live and
tested (`test_cor_do_m_pattern*`, cor.rs:470/578/590) — **not** dead code.

### 6. Utility macros absent from the README
- `map_insert!(m, { k: v, ... })` / `[ k => v, ... ]` (`src/common.rs`) — bulk
  `HashMap` population in a language without map literals (several syntaxes).
- `cor_newmutex!`, `cor_newmutex_and_start!`, `cor_yield!` (`src/cor.rs`) — the actual
  macros for building/driving coroutines.
- `contains!`, `reverse!` (`src/fp.rs`) — curried `Vec` helpers.
- `spread_and_call!`, `partial_left_last_one!` (`src/fp.rs`) — currying **plumbing**
  behind the fp macros; internal, not intended as user entry points.

## Scenarios & better ways to use

### MonadIO — deferred/lazy IO with scheduler hopping
*When:* defer a computation and run it off-thread, delivering the result on another
thread. *Better way:* set **both** `observe_on` and `subscribe_on` (see the trap in
§1); start both handlers before `subscribe`. Runnable: `examples/scheduler.rs`
(proves the routing and the `subscribe_on`-alone no-op by asserting `ThreadId`);
`examples/monadio.rs` shows the basic sync/async subscribe.

### Publisher — Rx pub/sub, three consumption styles
Callback (`subscribe_fn`), **pull** (`as_blocking_queue` → `take`), or **stream**
(`subscribe_as_stream` → `collect().await`). Choose pull when the consumer sets the
pace; stream when composing with `futures`. Runnable: `examples/publisher_queue.rs`
(push→pull bridge).

### Cor — coroutines & do-notation for sequencing
*When:* sequence dependent steps without nesting. Use `do_m!`/`do_m_pattern!`.
**Gotcha (verified invariant):** a `yield_from` target must be **async**
(`set_async(true)`, the default for `cor_newmutex_and_start!`). Two coroutines that
yield to each other while both sync **deadlock** — keep the entry coroutine sync and
targets async (see README "Cor sync deadlock invariant" and `src/cor.rs`).

### Actor — Akka-style ask/tell with children
`examples/actor.rs` (spawn + child lifecycle) and `examples/actor_ask.rs` (reply via a
`BlockingQueue`, using a take-timeout, not sleep). Shutdown is per-actor and manual —
do not assume structured teardown.

### Anti-patterns (grounded in this codebase)
1. **`thread::sleep` for synchronization** → use `CountDownLatch::wait()`, a
   `BlockingQueue` take, or `to_future().await`. (This session removed 7 such sleeps;
   examples are sleep-free.)
2. **`subscribe_on` without `observe_on` in MonadIO** → silent no-op (§1).
3. **Sync `yield_from` target** → deadlock; keep targets async.

## Cross-language family `[INFERENCE]`
`fpGo` (Go) and `fpEs` (ECMAScript) are the sibling ports; `Cor` was ported "from
golang version". Specific fpGo/fpEs API details are **inferred** from this repo's
comments/commits and are not verified against those repositories here.

## Provenance (attributions)
Haskell (monads, do-notation) · Java `java.util.concurrent` (`BlockingQueue`,
`CountDownLatch`, `Will`/future) · Android `Handler` loop (`HandlerThread`) · RxJava
schedulers (`observe_on`/`subscribe_on`) · Go coroutines (`Cor`) · Akka/Erlang
(`ActorAsync` ask) · `dtolnay/reduce` (`Reduce`) · `eliovir/rust-examples` (Observer).
The `compose!`/`pipe!` macros were reimplemented from first principles this session to
remove a CC BY-SA snippet. See [CONTRIBUTING](../CONTRIBUTING.md#source-credit-and-provenance).
