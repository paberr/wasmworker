use std::cell::Cell;

use wasm_bindgen::{prelude::wasm_bindgen, UnwrapThrowExt};

use super::WebWorkerPool;

/// This enumeration contains the supported strategies for distributing
/// tasks within the worker pool.
///
/// If re-exported, the strategy can also be accessed from JavaScript.
/// Rust:
/// ```rust
/// pub use wasmworker::pool::Strategy;
/// ```
#[wasm_bindgen]
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// The round-robin strategy will allocate tasks in a round-robin fashion
    /// to the workers in the pool.
    #[default]
    RoundRobin,
    /// The load-based strategy will allocate a task always to the worker with
    /// the lowest number of tasks already scheduled.
    /// If more than one worker has the same number of tasks scheduled, the first
    /// one is chosen.
    LoadBased,
}

/// The internal scheduler object, which contains necessary additional state
/// for the scheduling.
pub(super) struct Scheduler {
    /// The chosen strategy.
    strategy: Strategy,
    /// The currently chosen worker.
    /// This state is only relevant for the round-robin strategy.
    current_worker: Cell<usize>,
}

impl Scheduler {
    /// Initialize a new scheduler.
    pub(super) fn new(strategy: Strategy) -> Self {
        Self {
            strategy,
            current_worker: Cell::new(0),
        }
    }

    /// Given the pool, apply the strategy and determine which worker
    /// should receive the next task.
    pub(super) fn schedule(&self, pool: &WebWorkerPool) -> usize {
        match self.strategy {
            Strategy::RoundRobin => {
                // Simply return the current worker and increment.
                let worker_id = self.current_worker.get();
                self.current_worker
                    .set((worker_id + 1) % pool.num_workers());
                worker_id
            }
            Strategy::LoadBased => {
                // Choose the worker with the minimum work load.
                pool.workers
                    .iter()
                    .enumerate()
                    .min_by_key(|(_id, worker)| worker.current_load())
                    .expect_throw("WorkerPool does not have workers")
                    .0
            }
        }
    }
}
