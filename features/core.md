# Feature: core

`source_commit: 18a96af` · `last_verified_at: 2026-09-23` · `verification_status: passed`

User goal: Optional/Maybe semantics, fp function helpers (map/reduce/filter/
fold/compose/pipe/partial/spread), common helpers (LinkedListAsync,
SubscriptionFunc, RawFunc/RawReceiver, get_mut, generate_id).

## Preconditions

- Repo root; `cargo test` works (VERIFY.md → Launch, Doctor).

## Entry points

| Area | Files | Tests |
| --- | --- | --- |
| Maybe | `src/maybe.rs` | `maybe::test_*` (17) + doc tests |
| fp functions | `src/fp.rs` | `fp::test_*` (23) + doc tests |
| common helpers | `src/common.rs` | `common::test_*` (14) |

## Drive

```bash
cargo test maybe::
cargo test fp::
cargo test common::
```

## Observable outcomes

- `Maybe::just(1)` present; `Maybe::val()` unwraps; `map`/`ap`/`just` doc
  tests pin the API shape.
- fp: `compose`/`pipe` are mirrors (`test_fp_compose_pipe_are_mirror`);
  `partial_left_last_one`; `spread_and_call`; `fp!` macro curried forms.
- common: `LinkedListAsync` FIFO + clone-shares-state; `SubscriptionFunc`
  clone-equal-same-id / distinct-ids; `RawFunc`/`RawReceiver` invoke/pass.

## Failure paths

- `just` of a value reporting absent → presence contract broken.
- `compose`/`pipe` no longer mirroring → ordering contract broken.
- `LinkedListAsync` clone not sharing state or draining out of order →
  shared-state FIFO broken.
- `SubscriptionFunc` clones comparing unequal → identity contract broken.

## Evidence

- `cargo test <module> -- --nocapture` output; doc tests are part of the
  oracle — a doc-test failure is a product failure, not a doc nit.
