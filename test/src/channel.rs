use serde::{Deserialize, Serialize};
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::JsFuture;
use wasmworker::webworker_channel_fn;
use wasmworker::{
    webworker, webworker_channel, worker_pool, Channel, TaskError, WebWorker, WebWorkerPool,
    WorkerPoolOptions,
};

use crate::{js_assert_eq, raw::sort};

async fn sleep_ms(ms: u32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        web_sys::window()
            .unwrap_throw()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms as i32)
            .unwrap_throw();
    });
    JsFuture::from(promise).await.unwrap_throw();
}

/// Progress message sent from worker to main thread.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Progress {
    pub percent: u8,
}

/// Confirmation message sent from main thread to worker.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Continue {
    pub should_continue: bool,
}

/// Result of the processing.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProcessResult {
    pub items_processed: usize,
    pub was_cancelled: bool,
}

/// A simple async function that sends progress via the channel.
#[webworker_channel_fn]
pub async fn process_with_progress(data: Vec<u8>, channel: Channel) -> ProcessResult {
    let total = data.len();
    let mut processed = 0;

    for (i, _item) in data.iter().enumerate() {
        // Report progress at 50%
        if i == total / 2 {
            channel.send(&Progress { percent: 50 });

            // Wait for confirmation to continue
            let response: Option<Continue> = channel.recv().await;
            if let Some(cont) = response {
                if !cont.should_continue {
                    return ProcessResult {
                        items_processed: processed,
                        was_cancelled: true,
                    };
                }
            }
        }

        processed += 1;
    }

    // Report completion
    channel.send(&Progress { percent: 100 });

    ProcessResult {
        items_processed: processed,
        was_cancelled: false,
    }
}

/// Test that channel functions work with a single WebWorker.
pub(crate) async fn can_use_channel_with_worker() {
    let worker = WebWorker::new(None).await.expect("Couldn't create worker");

    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    let task = worker
        .run_channel(webworker_channel!(process_with_progress), &data)
        .await;

    // Wait for 50% progress
    let progress: Progress = task.recv().await.expect("Should receive 50% progress");
    js_assert_eq!(progress.percent, 50, "Should be at 50%");

    // Tell the worker to continue
    task.send(&Continue {
        should_continue: true,
    });

    // Wait for 100% progress
    let final_progress: Progress = task.recv().await.expect("Should receive 100% progress");
    js_assert_eq!(final_progress.percent, 100, "Should be at 100%");

    // Now wait for the task result
    let result = task.result().await.expect("Channel task should succeed");
    js_assert_eq!(result.items_processed, 10, "Should process all items");
    js_assert_eq!(result.was_cancelled, false, "Should not be cancelled");
}

/// Test that channel functions work with cancellation.
pub(crate) async fn can_cancel_channel_task() {
    let worker = WebWorker::new(None).await.expect("Couldn't create worker");

    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    let task = worker
        .run_channel(webworker_channel!(process_with_progress), &data)
        .await;

    // Wait for 50% progress
    let progress: Progress = task.recv().await.expect("Should receive 50% progress");
    js_assert_eq!(progress.percent, 50, "Should be at 50%");

    // Tell the worker to cancel
    task.send(&Continue {
        should_continue: false,
    });

    // Wait for result (no 100% progress expected since we cancelled)
    let result = task.result().await.expect("Channel task should succeed");
    js_assert_eq!(result.items_processed, 5, "Should process half the items");
    js_assert_eq!(result.was_cancelled, true, "Should be cancelled");
}

/// Test that worker termination is reported through the channel task result.
pub(crate) async fn channel_task_reports_worker_termination() {
    let worker = WebWorker::new(None).await.expect("Couldn't create worker");
    let data = vec![1, 2, 3, 4];
    let task = worker
        .run_channel(webworker_channel!(process_with_progress), &data)
        .await;
    let _: Progress = task.recv().await.expect("Should receive progress");

    drop(worker);
    let was_terminated = matches!(task.result().await, Err(TaskError::WorkerTerminated));
    js_assert_eq!(
        was_terminated,
        true,
        "Terminated worker should fail its channel task"
    );
}

/// Test that external termination wakes a task blocked in recv().
pub(crate) async fn external_termination_wakes_channel_recv() {
    let pool = WebWorkerPool::with_num_workers(1)
        .await
        .expect("Couldn't create worker pool");
    let data = vec![1, 2, 3, 4];
    let task = pool
        .run_channel(webworker_channel!(process_with_progress), &data)
        .await;
    let _: Progress = task.recv().await.expect("Should receive progress");
    let control = task.control();

    let recv_woke = std::rc::Rc::new(std::cell::Cell::new(false));
    let recv_woke_task = std::rc::Rc::clone(&recv_woke);
    wasm_bindgen_futures::spawn_local(async move {
        recv_woke_task.set(task.recv::<Progress>().await.is_none());
    });

    sleep_ms(10).await;
    control.terminate();
    while !recv_woke.get() {
        sleep_ms(10).await;
    }

    let input: Box<[u8]> = vec![3, 1, 2].into();
    let sorted = pool.run_bytes(webworker!(sort), &input).await;
    js_assert_eq!(sorted, Box::<[u8]>::from([1, 2, 3]));
}

/// Test that external termination wakes a task blocked in result().
pub(crate) async fn external_termination_wakes_channel_result() {
    let pool = WebWorkerPool::with_num_workers(1)
        .await
        .expect("Couldn't create worker pool");
    let data = vec![1, 2, 3, 4];
    let task = pool
        .run_channel(webworker_channel!(process_with_progress), &data)
        .await;
    let _: Progress = task.recv().await.expect("Should receive progress");
    let control = task.control();

    let result_woke = std::rc::Rc::new(std::cell::Cell::new(false));
    let result_woke_task = std::rc::Rc::clone(&result_woke);
    wasm_bindgen_futures::spawn_local(async move {
        result_woke_task.set(matches!(
            task.result().await,
            Err(TaskError::WorkerTerminated)
        ));
    });

    sleep_ms(10).await;
    control.terminate();
    control.terminate();
    while !result_woke.get() {
        sleep_ms(10).await;
    }

    let input: Box<[u8]> = vec![3, 1, 2].into();
    let sorted = pool.run_bytes(webworker!(sort), &input).await;
    js_assert_eq!(sorted, Box::<[u8]>::from([1, 2, 3]));
}

/// Test that channel functions work with the worker pool.
pub(crate) async fn can_use_channel_with_pool() {
    let pool = worker_pool().await;

    let data = vec![1, 2, 3, 4];

    let task = pool
        .run_channel(webworker_channel!(process_with_progress), &data)
        .await;

    // Wait for 50% progress
    let progress: Progress = task.recv().await.expect("Should receive 50% progress");
    js_assert_eq!(progress.percent, 50, "Should be at 50%");

    // Tell the worker to continue
    task.send(&Continue {
        should_continue: true,
    });

    // Wait for 100% progress
    let final_progress: Progress = task.recv().await.expect("Should receive 100% progress");
    js_assert_eq!(final_progress.percent, 100, "Should be at 100%");

    // Wait for completion
    let result = task.result().await.expect("Channel task should succeed");
    js_assert_eq!(result.items_processed, 4, "Should process all items");
    js_assert_eq!(result.was_cancelled, false, "Should not be cancelled");
}

/// Test that successful completion disarms termination before dropping the task.
pub(crate) async fn successful_channel_result_keeps_worker_active() {
    let pool = WebWorkerPool::with_num_workers(1)
        .await
        .expect("Couldn't create worker pool");
    let data = vec![1, 2, 3, 4];
    let task = pool
        .run_channel(webworker_channel!(process_with_progress), &data)
        .await;
    let _: Progress = task.recv().await.expect("Should receive progress");
    task.send(&Continue {
        should_continue: true,
    });
    let _: Progress = task.recv().await.expect("Should receive final progress");
    let _ = task.result().await.expect("Channel task should succeed");

    js_assert_eq!(
        pool.num_active_workers(),
        1,
        "Successful completion should not terminate its worker"
    );
}

/// Test that a pool channel task exclusively leases its worker.
pub(crate) async fn channel_task_exclusively_leases_worker() {
    let pool = std::rc::Rc::new(
        WebWorkerPool::with_num_workers(1)
            .await
            .expect("Couldn't create worker pool"),
    );
    let data = vec![1, 2, 3, 4];
    let task = pool
        .run_channel(webworker_channel!(process_with_progress), &data)
        .await;
    let _: Progress = task.recv().await.expect("Should receive progress");

    let queued_pool = std::rc::Rc::clone(&pool);
    let queued_result = std::rc::Rc::new(std::cell::RefCell::new(None));
    let task_result = std::rc::Rc::clone(&queued_result);
    wasm_bindgen_futures::spawn_local(async move {
        let input: Box<[u8]> = vec![3, 1, 2].into();
        let result = queued_pool.run_bytes(webworker!(sort), &input).await;
        *task_result.borrow_mut() = Some(result);
    });

    sleep_ms(50).await;
    js_assert_eq!(
        queued_result.borrow().is_none(),
        true,
        "Ordinary task should wait for channel lease"
    );

    task.send(&Continue {
        should_continue: true,
    });
    let _ = task
        .result()
        .await
        .expect("Channel task should release its worker");
    while queued_result.borrow().is_none() {
        sleep_ms(10).await;
    }
    let sorted = queued_result
        .borrow_mut()
        .take()
        .expect("Queued task should complete");
    js_assert_eq!(sorted, Box::<[u8]>::from([1, 2, 3]));
}

/// Test that explicit termination replaces the leased worker before reuse.
pub(crate) async fn terminating_channel_task_replaces_worker() {
    let mut options = WorkerPoolOptions::new();
    options.num_workers = Some(1);
    options.precompile_wasm = Some(true);
    let pool = WebWorkerPool::with_options(options)
        .await
        .expect("Couldn't create worker pool");
    let data = vec![1, 2, 3, 4];
    let task = pool
        .run_channel(webworker_channel!(process_with_progress), &data)
        .await;
    let _: Progress = task.recv().await.expect("Should receive progress");

    let control = task.control();
    control.terminate();
    control.terminate();
    js_assert_eq!(
        pool.num_active_workers(),
        0,
        "Terminated slot should not be schedulable during replacement"
    );
    let was_terminated = matches!(task.result().await, Err(TaskError::WorkerTerminated));
    js_assert_eq!(was_terminated, true);

    let input: Box<[u8]> = vec![3, 1, 2].into();
    let sorted = pool.run_bytes(webworker!(sort), &input).await;
    js_assert_eq!(sorted, Box::<[u8]>::from([1, 2, 3]));
    js_assert_eq!(pool.num_active_workers(), 1);
}

/// Test that dropping an unfinished channel task also replaces its worker.
pub(crate) async fn dropping_channel_task_replaces_worker() {
    let pool = WebWorkerPool::with_num_workers(1)
        .await
        .expect("Couldn't create worker pool");
    let data = vec![1, 2, 3, 4];
    let task = pool
        .run_channel(webworker_channel!(process_with_progress), &data)
        .await;
    let _: Progress = task.recv().await.expect("Should receive progress");

    drop(task);
    js_assert_eq!(
        pool.num_active_workers(),
        0,
        "Dropped task should make its slot unavailable during replacement"
    );

    let input: Box<[u8]> = vec![3, 1, 2].into();
    let sorted = pool.run_bytes(webworker!(sort), &input).await;
    js_assert_eq!(sorted, Box::<[u8]>::from([1, 2, 3]));
}
