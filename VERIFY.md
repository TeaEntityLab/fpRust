# VERIFY — fpRust

How an agent launches, checks, drives, and cleans up verification of this
library. Read this plus the relevant `features/*.md` file before touching the
code.

`source_commit: 18a96af` · `last_verified_at: 2026-09-23` · `verification_status: passed`

## Launch

```bash
cd <repo root>
cargo test
```

Runs 120 unit tests (in-module `#[test]` blocks under `src/`) plus 24 doc
tests — ~10s total. `test.sh` and `paralleltest.sh` are legacy wrappers;
`cargo test` is the canonical surface. `clippy.sh` runs the linter.

## Doctor

```bash
cargo test maybe::test_maybe -- --exact   # smoke: ~1s (name a real test)
cargo build
```

A healthy tree: smoke test PASS, `cargo build` clean, full suite
`120 passed; 0 failed` + `24 passed` doc tests.

## Drive

- Per feature: run `cargo test <module>::` (e.g. `cargo test fp::`) from the
  feature file's Drive section.
- For behavior not covered by a test, write a scratch driver **outside** the
  repo: `cargo new /tmp/fprust-driver`, add to its `Cargo.toml`:

  ```toml
  [dependencies]
  fp_rust = { path = "<repo root>" }
  futures = "0.3"
  ```

  then `cargo run` — never add driver files inside the repo.

## Evidence

- Save `cargo test <module> -- --nocapture` output to files under a scratch
  dir (e.g. `/tmp/fprust-evidence/`).
- For scratch drivers, record the driver source plus its stdout.

## When a check fails

Classify before fixing — four different failures need four different repairs:

| Class | Meaning | Repair |
| --- | --- | --- |
| Product regression | Library behavior changed for the worse | Report; do not edit the map or tests to match |
| Doc drift | The map/VERIFY.md no longer matches the library | Update the doc; the product is fine |
| Spec/oracle error | The acceptance spec or test expectation is wrong | Oracles are read-only; propose the change for review |
| Harness failure | Toolchain, cargo, target/, or environment broke | Fix the harness; the product is fine |

Never "fix" a failing check by editing the map or a test to describe the new
behavior — that hides product regressions. If the new behavior is intentional,
the map/test update is a separate reviewed change.

## Cleanup

- Remove scratch driver dirs and evidence dirs you created.
- `git status` must show no modifications inside the repo.
