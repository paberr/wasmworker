#![doc = include_str!("../README.md")]
#![allow(clippy::borrowed_box)]
pub use global::{has_worker_pool, init_worker_pool, worker_pool};
pub use pool::WorkerPoolOptions;
pub use webworker::WebWorker;

pub mod convert;
pub mod error;
pub mod func;
mod global;
#[cfg(feature = "serde")]
pub mod iter_ext;
pub mod pool;
mod webworker;
