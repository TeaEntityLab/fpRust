# fpRust — Project Notes: Origins, Identity & Less-Known Capabilities

> Durable engineering notes derived from a git-history + API-surface investigation
> (206 commits, 2018‑07 → 2026‑07). This document records *what kind of project this
> is*, *why it is shaped the way it is*, and the *public capabilities that are easy to
> miss*. It is context for maintainers and advanced users; the [README](../README.md)
> remains the primary user guide and [ROADMAP](../ROADMAP.md) the maintenance posture.

## Identity

fpRust is a **published polyglot functional-programming / reactive-pattern
library** — the Rust member of the **`fpGo`** (Go) / **`fpEs`** (ECMAScript) family.
Its APIs deliberately mirror those siblings (`src/maybe.rs:10` documents the
`map`/`chain`/`ap` parity), so the same FP/reactive concepts read the same way across
Go, JavaScript, and Rust. It is aimed at **learning, prototyping, and selective module
adoption** (an external user adopted its `BlockingQueue` — see Scenarios).

It is **not** a production async runtime or a Tokio/Actix competitor. It optimizes for
**clarity and teachability** of cross-language patterns, and is a good fit for
*learning & prototyping* and *selective module use* — not *production orchestration*
(no unified shutdown protocol; see [ROADMAP](../ROADMAP.md) exclusions).

## Origin story (three eras)

| Era | Window | What landed | Influences |
|-----|--------|-------------|------------|
| **1 — Sync FP + threading core** | 2018‑07 (142 commits) | `Maybe` → `MonadIO` → `Handler`/`HandlerThread` → `BlockingQueue` → `Publisher`/`SubscriptionFunc` → `CountDownLatch` → `Cor` → fp macros (`map!`/`reduce!`/`filter!`/`fold*!`/`compose!`/`pipe!`) → `do_m!` | **Commit-cited:** Go coroutines (Cor, `28f8178`), `dtolnay/reduce` (`0a53ac6`), Handler threading (Stack Overflow `1f15ba1`/`93a36f8`), GoF Observer (`eliovir/rust-examples`, source comment since `9932452`). **Naming-only [INFERENCE]:** Haskell monads/do‑notation, Android `Handler` loop, Java `java.util.concurrent`, RxJava schedulers |
| **2 — Java Future** | 2018‑08 (3 commits) | `WillAsync` ("to achieve JavaFuture") | Java `Future` callback/wait semantics |
| **3 — Async Rust + Actors** | 2021‑06→08 (56 commits) | Rust‑2021 modernization (only external PR ever, #1 `bogdad/fix_compilation`), `futures::Stream` + `Future` integration, `subscribe_as_stream`, real async `CountDownLatch`, `BlockingQueue::{take,poll}_result_as_future`, **Akka‑style `ActorAsync`** (ask/tell + children), `LinkedListAsync`, feature‑flag modularization |

(2026‑07 added test coverage, docs/roadmap, and async‑waker correctness fixes — see [CHANGELOG](../CHANGELOG.md).)

### On the word "leaked"
The 2018‑07‑11 commit **"Add leaked observe/subscribe_on"** (`b2e17e5`, +8 lines to
`monadio.rs`, no tests/docs) is the author's own term for **capabilities shipped ahead
of their documentation** — public, working, but never surfaced to users. This document
is largely about paying down that gap. (Distinct from the *resource*-leak commits
`6d801ad` "potential leaked future/stream.wake()" and `40ef803` "SubscriptionFunc
buffer leak" — same word, unrelated meaning.)

## Less-known / "leaked" capabilities

These are fully public and working. README §"Less-known / advanced capabilities" now
summarizes items 1–5 and most of §6; this section keeps provenance, traps, scenarios,
and syntax-level detail. Ranked by user value; every claim traced to source.

### 1. `MonadIO::observe_on` / `subscribe_on` — scheduler routing (with a trap)
`src/monadio.rs:105,110`. RxJava-like naming (`observeOn`/`subscribeOn`; naming-only
[INFERENCE]), but the semantics **differ** — verified in `MonadIO::subscribe` at
`src/monadio.rs:141‑163`:

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
tested (`test_cor_do_m_pattern*`) — **not** dead code.

### 6. Utility macros (detail beyond README)
- `map_insert!(m, { k: v, ... })` / `[ k => v, ... ]` (`src/common.rs`) — bulk
  `HashMap` population in a language without map literals (brace `k: v`, bracket
  `k => v`, and comma-separated `k, v` forms).
- `cor_newmutex!`, `cor_newmutex_and_start!`, `cor_yield!` (`src/cor.rs`) — the actual
  macros for building/driving coroutines (`cor_newmutex!` omitted from README advanced list).
- `contains!`, `reverse!` (`src/fp.rs`) — curried `Vec` helpers.
- `spread_and_call!`, `partial_left_last_one!` (`src/fp.rs:62,80`) — `#[macro_export]`
  currying primitives with doctests; lower-level than `map!`/`filter!` but public.

### 7. Additional public APIs easy to miss
- **`monadio::of` / `From<Y> for MonadIO<Y>`** (`src/monadio.rs:39,45`) — lift pure
  values; `just` is sugar over `new(of(r))`.
- **`Publisher::map`** (`src/publisher.rs:83`) — **side-effect** subscriber: runs `func`
  on each publish; return value is **discarded** (not a functor map).
- **`Cor::yield_from` cross-type yield** (`src/cor.rs:215`) — `Cor<RETURN, RECEIVE>`
  may delegate to `Cor<RETURNTARGET, RECEIVETARGET>`; yield **out** type and resume
  **in** type can differ across the call.
- **`LinkedListAsync` as `futures::Stream`** (`src/common.rs:224`, `for_futures`) —
  backing type for `Publisher::subscribe_as_stream` / `SubscriptionFunc::as_stream`.
- **`RawFunc` / `RawReceiver`** (`src/common.rs:509,553`) — `Handler::post` plumbing
  wrapping `FnMut` closures; surfaced when reading scheduler routing.

## Scenarios & better ways to use

### BlockingQueue — standalone `java.util.concurrent`-style queue (selective use)
*When:* you need a cloneable, thread-safe producer/consumer queue with blocking `take`,
optional `timeout`, and `offer`/`poll` — without pulling in Actor, Publisher, or MonadIO.
It is the crate's most reused primitive: it backs `HandlerThread`, actor mailboxes, and
the Publisher push→pull bridge. *Better way:* treat it as an **unbounded** channel
wrapper (`src/sync.rs` — `put` has no max); set `timeout` on consumers that must not
block forever; join with `CountDownLatch`/`take()`, never `thread::sleep`. *Demand is
real:* the only external PR (#1, `bogdad`) adopted fpRust specifically as a "java like
blocking array queue" (observed in the PR description via GitHub; the PR *diff* itself
is only a `reduce` compile fix, not queue code).

### MonadIO — deferred/lazy IO with scheduler hopping
*When:* defer a computation and run it off-thread, delivering the result on another
thread. *Better way:* set **both** `observe_on` and `subscribe_on` (see the trap in
§1); start both handlers before `subscribe`. Runnable: `examples/scheduler.rs`
(proves the routing and the `subscribe_on`-alone no-op by asserting `ThreadId`);
`examples/monadio.rs` shows the basic sync/async subscribe.

### Publisher — Rx pub/sub, three consumption styles
Callback (`subscribe_fn`), **pull** (`as_blocking_queue` → `take`), or **stream**
(`subscribe_as_stream` → `collect().await`). Choose pull when the consumer sets the
pace; stream when composing with `futures`. Runnable: `examples/publisher.rs`
(callback), `examples/publisher_queue.rs` (push→pull). The stream style is covered by
doctests/tests under `for_futures` only — no dedicated `examples/*.rs` yet.

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
   `BlockingQueue` take, or `to_future().await`. (The CHANGELOG documents removal of
   legacy test sleeps; all `examples/*.rs` are sleep-free.)
2. **`subscribe_on` without `observe_on` in MonadIO** → silent no-op (§1).
3. **Sync `yield_from` target** → deadlock; keep targets async.
4. **Treating `BlockingQueue` as bounded or interruptible** → `put` is **unbounded**
   (no max size) and `stop()` does **not** unblock a thread already in `take()`; give
   consumers an explicit `timeout` instead.

## Cross-language family
**Observed:** the parity *intent* is source-documented — `src/maybe.rs:10` states the
API "mirrors sister libraries (`fpEs`, `fpGo`)", and `Cor` was ported "from golang
version" (`28f8178`). **[INFERENCE]:** the specific fpGo/fpEs APIs are not fetched or
verified against those repositories here.

## Provenance (attributions)
**Commit-cited:** Go coroutines (`Cor`, `28f8178`) · Java Future (`WillAsync`, `24060c2`) ·
`dtolnay/reduce` (`Reduce`, `0a53ac6` + `fp.rs:317`) · Handler/`RawFunc` (Stack Overflow
`1f15ba1`/`93a36f8`) · GoF Observer (`eliovir/rust-examples`, source comment since `9932452`
→ `common.rs:282`) · Akka-style ask (`ActorAsync`, `d173df7`). **Naming-only [INFERENCE]
(no in-repo citation):** Haskell monads/do-notation · Java `java.util.concurrent`
(`BlockingQueue`/`CountDownLatch`) · Android `Handler` loop · RxJava schedulers
(`observe_on`/`subscribe_on`). The `compose!`/`pipe!` macros were reimplemented from first
principles to remove a CC BY-SA snippet. See [CONTRIBUTING](../CONTRIBUTING.md#source-credit-and-provenance).
