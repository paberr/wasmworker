use serde::{Deserialize, Serialize};
use webworker::{iter_ext::IteratorExt, webworker, worker_pool, WebWorker};
use webworker_proc_macro::webworker_fn;

use crate::js_assert_eq;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct VecType(Vec<u8>);

#[webworker_fn]
pub fn sort_vec(mut v: VecType) -> VecType {
    v.0.sort();
    v
}

pub(crate) async fn can_run_task() {
    let worker = WebWorker::new(None).await.expect("Couldn't create worker");

    // Prepare input and output.
    let vec = VecType(vec![8, 1, 5, 0, 4]);
    let mut sorted_vec = vec.0.clone();
    sorted_vec.sort();
    let sorted_vec = VecType(sorted_vec);

    // Test try run.
    let res1 = worker
        .try_run(webworker!(sort_vec), &vec)
        .await
        .expect("Should not be full");
    js_assert_eq!(res1, sorted_vec, "Raw try run failed");

    // Test run.
    let res2 = worker.run(webworker!(sort_vec), &vec).await;
    js_assert_eq!(res2, sorted_vec, "Raw run failed");
}

pub(crate) async fn can_limit_tasks() {
    let worker = WebWorker::new(Some(0))
        .await
        .expect("Couldn't create worker");

    // Prepare input.
    let vec = VecType(vec![8, 1, 5, 0, 4]);

    // Test try run.
    let res1 = worker.try_run(webworker!(sort_vec), &vec).await;
    match res1 {
        Ok(_) => wasm_bindgen::throw_str("Should not be able to obtain permit"),
        Err(_) => {}
    }
}

pub(crate) async fn can_schedule_task() {
    let pool = worker_pool().await;

    // Prepare input and output.
    let vec = VecType(vec![8, 1, 5, 0, 4]);
    let mut sorted_vec = vec.0.clone();
    sorted_vec.sort();
    let sorted_vec = VecType(sorted_vec);

    // Test try run.
    let res1 = pool
        .try_run(webworker!(sort_vec), &vec)
        .await
        .expect("Should not be full");
    js_assert_eq!(res1, sorted_vec);

    // Test run.
    let res2 = pool.run(webworker!(sort_vec), &vec).await;
    js_assert_eq!(res2, sorted_vec);
}

pub(crate) async fn can_use_iter_ext() {
    // Prepare input and output.
    let vec = vec![
        VecType(vec![8, 1, 5, 0, 4]),
        VecType(vec![8, 2, 5, 0, 4]),
        VecType(vec![8, 1, 7, 0, 4]),
    ];
    let mut sorted_vec = vec.clone();
    for sub_vec in sorted_vec.iter_mut() {
        sub_vec.0.sort();
    }

    // Test iter.
    let res1 = vec.iter().par_map(webworker!(sort_vec)).await;
    js_assert_eq!(res1, sorted_vec);

    // Test into_iter.
    let res2 = vec.into_iter().par_map(webworker!(sort_vec)).await;
    js_assert_eq!(res2, sorted_vec);
}
