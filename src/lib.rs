// #[cfg(feature = "for_futures")]
// extern crate tokio;
#[cfg(feature = "for_futures")]
extern crate futures;

pub mod cor;
pub mod fp;
pub mod handler;
pub mod maybe;
pub mod monadio;
pub mod publisher;
pub mod sync;

pub mod common;
