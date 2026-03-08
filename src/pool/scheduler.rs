use std::cell::Cell;

use wasm_bindgen::prelude::wasm_bindgen;

/// This enumeration contains the supported strategies for distributing
/// tasks within the worker pool.
///
/// If re-exported, the strategy can also be accessed from JavaScript.
/// Rust:
/// ```rust
/// pub use wasmworker::pool::Strategy;
/// ```
#[non_exhaustive] // forward compatibility
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

    /// Given per-slot loads, apply the strategy and determine which worker
    /// should receive the next task. Returns `None` if no active workers exist.
    ///
    /// Each entry in `loads` is `Some(current_load)` for active workers,
    /// or `None` for terminated/creating slots.
    pub(super) fn schedule(&self, loads: &[Option<usize>]) -> Option<usize> {
        match self.strategy {
            Strategy::RoundRobin => {
                let num = loads.len();
                for _ in 0..num {
                    let id = self.current_worker.get();
                    self.current_worker.set((id + 1) % num);
                    if loads[id].is_some() {
                        return Some(id);
                    }
                }
                None
            }
            Strategy::LoadBased => loads
                .iter()
                .enumerate()
                .filter_map(|(i, load)| load.map(|l| (i, l)))
                .min_by_key(|(_i, load)| *load)
                .map(|(i, _)| i),
        }
    }
}
