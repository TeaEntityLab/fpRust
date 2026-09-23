# Feature: reactive

`source_commit: 18a96af` · `last_verified_at: 2026-09-23` · `verification_status: passed`

User goal: Rx-like MonadIO streams and Publisher pub/sub.

## Preconditions

- Repo root; `cargo test` works.

## Entry points

| Area | Files | Tests |
| --- | --- | --- |
| MonadIO | `src/monadio.rs` | `monadio::test_*` (13) |
| Publisher | `src/publisher.rs` | `publisher::test_*` (14) |

## Drive

```bash
cargo test monadio::
cargo test publisher::
```

## Observable outcomes

- MonadIO `of` returns value; `subscribe`/`subscribe_fn` deliver via on_next.
- Publisher: `publish` notifies all subscribers; `map` runs on publish;
  `unsubscribe` stops delivery to the correct observer only; subscribe
  returns the same handle for the same observer; blocking-queue subscription
  mode works.

## Failure paths

- `unsubscribe` removing the wrong observer or not stopping delivery →
  observer-registry broken.
- `map` not running on publish → transform chain broken.
- Multiple subscribers not all notified → fanout broken.
- Blocking-queue mode losing values → queue bridge broken.

## Evidence

- `cargo test <module> -- --nocapture` output.
