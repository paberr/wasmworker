#![doc = include_str!("../README.md")]
#![allow(clippy::borrowed_box)]
pub use channel::Channel;
pub use global::{has_worker_pool, init_worker_pool, worker_pool, AlreadyInitialized};
pub use pool::WorkerPoolOptions;
pub use web_sys::MessagePort;
pub use webworker::WebWorker;

// Re-export WebWorkerPool from pool module
pub use pool::WebWorkerPool;

mod channel;
pub mod convert;
pub mod error;
pub mod func;
mod global;
#[cfg(feature = "serde")]
pub mod iter_ext;
pub mod pool;
mod webworker;
