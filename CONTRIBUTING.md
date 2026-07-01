# Contributing to fpRust

Thank you for your interest in improving fpRust. This project is
[passively maintained](https://crates.io/crates/fp_rust); contributions are
welcome but may not be reviewed immediately.

## Development setup

- Rust **stable** toolchain (see `.github/workflows/ci.yml`).
- Clone the repository: `git clone https://github.com/TeaEntityLab/fpRust.git`

## Running tests

Default library tests:

```bash
cargo test
```

Full runtime tests (async/`futures` integration):

```bash
cargo test --features=test_runtime
# or
./test.sh
```

Doc tests with the test runtime feature:

```bash
cargo test --doc --features=test_runtime
```

Example targets (when present):

```bash
cargo test --examples --features=test_runtime
```

### Stress / concurrency testing

`paralleltest.sh` runs `./test.sh` many times in parallel to surface flaky
timing or concurrency bugs. It caps concurrency at 10 subshells.

```bash
./paralleltest.sh
```

Filter output for failures:

```bash
./paralleltest.sh 2>&1 | grep failed
./paralleltest.sh 2>&1 | grep "assertion failed"
```

This is intentionally heavy (1000 iterations); use it when changing
`sync`, `handler`, `actor`, or other concurrent code paths.

### Linting

```bash
./clippy.sh
```

Runs `cargo clippy --all-targets --all-features` on stable.

## Source credit and provenance

fpRust draws API ideas from several ecosystems. When extending or documenting
code, preserve these attributions where relevant:

- **BlockingQueue / CountDownLatch / Future-style waiting** — inspired by Java
  concurrency primitives (`java.util.concurrent`).
- **HandlerThread** — inspired by Android's `Handler` / message-loop model.
- **Actor ask/reply** — inspired by Akka / Erlang actor patterns.
- **Coroutines** — Python generator-style `yield` / `yieldFrom` ergonomics.
- **Do notation** — Haskell-style `do` blocks (macro-based).
- **CI scaffolding** — an early `.travis.yml` was adapted from the
  [ripgrep](https://github.com/BurntSushi/ripgrep) project template (since
  replaced by GitHub Actions).
- **`pipe!` / `compose!` / `compose_two`** — concept adapted from functional
  composition patterns; reimplemented in-tree to avoid a CC BY-SA snippet (credit
  the idea only, no upstream code license).
- **`Reduce` trait** — adapted from [dtolnay/reduce](https://github.com/dtolnay/reduce)
  (MIT OR Apache-2.0).
- **`Observable` / `Subscription` observer pattern** — adapted from
  [eliovir/rust-examples](https://github.com/eliovir/rust-examples) (upstream
  license to be verified).

If you port algorithms or substantial code from another project, cite the
original source in commit messages and, when appropriate, in module-level
documentation.

## Release process

Publishing is handled manually via `publish.sh`. The script:

1. Lists packaged files (`cargo package --list`)
2. Builds a local package dry-run
3. Verifies the git tag `v<Cargo.toml version>` exists
4. Runs `cargo publish --dry-run`, then prompts before `cargo publish`

It does **not** auto-push commits or tags. After a successful publish:

```bash
git push
git push origin v<version>
```

Bump `version` in `Cargo.toml`, update `Cargo.lock` if needed, tag the release
(`git tag -a v<version> -m "Release <version>"`), then run `./publish.sh`.

## Pull requests

- Keep changes focused; match existing style in the touched modules.
- Run `./test.sh` (or the relevant `cargo test` commands above) before opening
  a PR.
- CI must pass on stable (see `.github/workflows/ci.yml`).
