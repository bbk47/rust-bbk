//! bbk v4 library surface.
//!
//! The crate ships both a binary (`src/main.rs`) and this library so the
//! protocol building blocks (encryption, yamux tunnel, proxy framing) can be
//! exercised directly from the integration tests under `tests/`.

pub mod client;
pub mod option;
pub mod proxy;
pub mod serve;
pub mod server;
pub mod transport;
pub mod tunnel;
pub mod utils;
