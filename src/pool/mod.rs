use std::borrow::Borrow;

use futures::future::join_all;
use js_sys::wasm_bindgen::{prelude::wasm_bindgen, UnwrapThrowExt};
use scheduler::Scheduler;
pub use scheduler::Strategy;
use serde::{Deserialize, Serialize};
use web_sys::window;

use crate::{
    error::{Full, InitError},
    func::WebWorkerFn,
    WebWorker,
};

mod scheduler;

#[wasm_bindgen(getter_with_clone)]
#[derive(Default, Clone)]
pub struct WorkerPoolOptions {
    pub path: Option<String>,
    pub strategy: Option<Strategy>,
    pub num_workers: Option<usize>,
}

#[wasm_bindgen]
impl WorkerPoolOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Default::default()
    }
}

impl WorkerPoolOptions {
    fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    fn strategy(&self) -> Strategy {
        self.strategy.unwrap_or_default()
    }

    fn num_workers(&self) -> usize {
        self.num_workers.unwrap_or_else(|| {
            window()
                .expect_throw("Window missing")
                .navigator()
                .hardware_concurrency() as usize
        })
    }
}

pub struct WebWorkerPool {
    workers: Vec<WebWorker>,
    scheduler: Scheduler,
}

impl WebWorkerPool {
    pub async fn new() -> Result<Self, InitError> {
        Self::with_options(WorkerPoolOptions::default()).await
    }

    pub async fn with_strategy(strategy: Strategy) -> Result<Self, InitError> {
        Self::with_options(WorkerPoolOptions {
            strategy: Some(strategy),
            ..Default::default()
        })
        .await
    }

    pub async fn with_num_workers(num_workers: usize) -> Result<Self, InitError> {
        Self::with_options(WorkerPoolOptions {
            num_workers: Some(num_workers),
            ..Default::default()
        })
        .await
    }

    pub async fn with_path(path: String) -> Result<Self, InitError> {
        Self::with_options(WorkerPoolOptions {
            path: Some(path),
            ..Default::default()
        })
        .await
    }

    pub async fn with_options(options: WorkerPoolOptions) -> Result<Self, InitError> {
        let worker_inits = (0..options.num_workers()).map(|_| {
            // Do not impose a task limit.
            WebWorker::with_path(options.path(), None)
        });
        let workers = join_all(worker_inits).await;
        let workers = workers.into_iter().collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            workers,
            scheduler: Scheduler::new(options.strategy()),
        })
    }

    #[cfg(feature = "serde")]
    pub async fn run<T, R>(&self, func: WebWorkerFn<T, R>, arg: &T) -> R
    where
        T: Serialize + for<'de> Deserialize<'de>,
        R: Serialize + for<'de> Deserialize<'de>,
    {
        self.run_internal(func, arg).await
    }

    #[cfg(feature = "serde")]
    pub async fn try_run<T, R>(&self, func: WebWorkerFn<T, R>, arg: &T) -> Result<R, Full>
    where
        T: Serialize + for<'de> Deserialize<'de>,
        R: Serialize + for<'de> Deserialize<'de>,
    {
        let worker_id = self.scheduler.schedule(self);
        self.workers[worker_id].try_run(func, arg).await
    }

    pub async fn run_bytes(
        &self,
        func: WebWorkerFn<Box<[u8]>, Box<[u8]>>,
        arg: &Box<[u8]>,
    ) -> Box<[u8]> {
        self.run_internal(func, arg).await
    }

    pub async fn try_run_bytes(
        &self,
        func: WebWorkerFn<Box<[u8]>, Box<[u8]>>,
        arg: &Box<[u8]>,
    ) -> Result<Box<[u8]>, Full> {
        let worker_id = self.scheduler.schedule(self);
        self.workers[worker_id].try_run_bytes(func, arg).await
    }

    pub(crate) async fn run_internal<T, R, A>(&self, func: WebWorkerFn<T, R>, arg: A) -> R
    where
        A: Borrow<T>,
        T: Serialize + for<'de> Deserialize<'de>,
        R: Serialize + for<'de> Deserialize<'de>,
    {
        let worker_id = self.scheduler.schedule(self);
        self.workers[worker_id]
            .run_internal(func, arg.borrow())
            .await
    }

    /// Return the number of tasks currently queued to this worker pool.
    pub fn current_load(&self) -> usize {
        self.workers.iter().map(WebWorker::current_load).sum()
    }

    pub fn num_workers(&self) -> usize {
        self.workers.len()
    }
}
