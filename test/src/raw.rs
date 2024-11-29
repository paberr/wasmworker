use wasm_bindgen::throw_str;
use wasmworker::{error::InitError, webworker, worker_pool, WebWorker};
use wasmworker_proc_macro::webworker_fn;

use crate::js_assert_eq;

#[webworker_fn]
pub fn sort(mut v: Box<[u8]>) -> Box<[u8]> {
    v.sort();
    v
}

pub(crate) async fn can_handle_invalid_paths() {
    let worker = WebWorker::with_path(Some("something"), None).await;
    if !matches!(worker, Err(InitError::WebWorkerModuleLoading(_))) {
        throw_str("Should have failed initialization with wrong path");
    }
}

pub(crate) async fn can_run_task_bytes() {
    let worker = WebWorker::new(None).await.expect("Couldn't create worker");

    // Prepare input and output.
    let vec = vec![8, 1, 5, 0, 4];
    let mut sorted_vec = vec.clone();
    sorted_vec.sort();
    let vec = vec.into();
    let sorted_vec = sorted_vec.into();

    // Test try run.
    let res1 = worker
        .try_run_bytes(webworker!(sort), &vec)
        .await
        .expect("Should not be full");
    js_assert_eq!(res1, sorted_vec, "Raw try run failed");

    // Test run.
    let res2 = worker.run_bytes(webworker!(sort), &vec).await;
    js_assert_eq!(res2, sorted_vec, "Raw run failed");
}

pub(crate) async fn can_limit_tasks_bytes() {
    let worker = WebWorker::new(Some(0))
        .await
        .expect("Couldn't create worker");

    // Prepare input.
    let vec = vec![8, 1, 5, 0, 4];
    let vec = vec.into();

    // Test try run.
    let res1 = worker.try_run_bytes(webworker!(sort), &vec).await;
    if res1.is_ok() {
        wasm_bindgen::throw_str("Should not be able to obtain permit")
    }
}

pub(crate) async fn can_schedule_task_bytes() {
    let pool = worker_pool().await;

    // Prepare input and output.
    let vec = vec![8, 1, 5, 0, 4];
    let mut sorted_vec = vec.clone();
    sorted_vec.sort();
    let vec = vec.into();
    let sorted_vec = sorted_vec.into();

    // Test run.
    let res2 = pool.run_bytes(webworker!(sort), &vec).await;
    js_assert_eq!(res2, sorted_vec);
}
