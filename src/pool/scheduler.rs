use std::cell::Cell;

use wasm_bindgen::{prelude::wasm_bindgen, UnwrapThrowExt};

use super::WebWorkerPool;

#[wasm_bindgen]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    RoundRobin,
    LoadBased,
}

impl Default for Strategy {
    fn default() -> Self {
        Strategy::RoundRobin
    }
}

pub(super) struct Scheduler {
    strategy: Strategy,
    current_worker: Cell<usize>,
}

impl Scheduler {
    pub(super) fn new(strategy: Strategy) -> Self {
        Self {
            strategy,
            current_worker: Cell::new(0),
        }
    }

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
