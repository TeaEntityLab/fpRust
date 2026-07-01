//! Functional programming utilities for Rust.
//!
//! This crate provides optional-value helpers ([`maybe`]), collection utilities
//! ([`fp`]), shared primitives ([`common`]), and optional feature-gated modules
//! for actors, coroutines, pub/sub, and reactive handlers.
//!
//! # Features
//!
//! The default `pure` feature enables the full synchronous stack — [`fp`],
//! [`maybe`], [`sync`], [`cor`], [`actor`], [`handler`], [`monadio`],
//! [`publisher`]; async/futures integration is opt-in via `for_futures` (or
//! `test_runtime`).

// The test suite uses `assert_eq!(expected, actual)` uniformly (expected-first),
// including `assert_eq!(true, x)` / `assert_eq!(false, x)`. This is a deliberate
// project convention, so silence clippy's bool_assert_comparison lint crate-wide.
#![allow(clippy::bool_assert_comparison)]
// Pre-existing public API / deliberate data-model choices, out of scope for
// mechanical lint cleanup: `get_mut(&mut Vec<T>)` is a public teaching helper
// signature; `ActorAsync` stores its context as `Box<HashMap>` and a boxed
// effect closure by design. Keep them explicit rather than churn the public API.
#![allow(clippy::ptr_arg, clippy::box_collection, clippy::type_complexity)]

#[cfg(feature = "for_futures")]
extern crate futures;

pub mod common;

#[cfg(feature = "handler")]
pub mod handler;
#[cfg(feature = "publisher")]
pub mod publisher;
#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "actor")]
pub mod actor;
#[cfg(feature = "cor")]
pub mod cor;
#[cfg(feature = "fp")]
pub mod fp;
#[cfg(feature = "maybe")]
pub mod maybe;
#[cfg(feature = "monadio")]
pub mod monadio;
