use send_wrapper::SendWrapper;
use tokio::sync::OnceCell;
use wasm_bindgen::{prelude::wasm_bindgen, UnwrapThrowExt};

use crate::pool::{WebWorkerPool, WorkerPoolOptions};

static WORKER_POOL: OnceCell<SendWrapper<WebWorkerPool>> = OnceCell::const_new();

#[wasm_bindgen]
pub async fn init_worker_pool(num_workers: usize, path: Option<String>) {
    WORKER_POOL
        .get_or_init(|| async move {
            SendWrapper::new(
                WebWorkerPool::with_options(WorkerPoolOptions {
                    num_workers: Some(num_workers),
                    path,
                    ..Default::default()
                })
                .await
                .expect_throw("Couldn't instantiate worker pool"),
            )
        })
        .await;
}

pub async fn worker_pool() -> &'static WebWorkerPool {
    WORKER_POOL
        .get_or_init(|| async {
            SendWrapper::new(
                WebWorkerPool::new()
                    .await
                    .expect_throw("Couldn't instantiate worker pool"),
            )
        })
        .await
}
