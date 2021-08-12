#[cfg(feature = "for_futures")]
extern crate futures;
// #[cfg(feature = "for_futures")]
// extern crate tokio;

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
