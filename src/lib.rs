#[cfg(feature = "for_futures")]
extern crate futures;
// #[cfg(feature = "for_futures")]
// extern crate tokio;

pub mod handler;
pub mod publisher;
pub mod sync;

pub mod common;

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
