pub mod setup;

/// An older API, provided for compatibility.
pub mod old;

pub use setup::{init_tracing, GlobalTracing};
