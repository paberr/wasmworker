use futures::future::select;
use futures::future::Either;
use futures::pin_mut;
use serde::{Deserialize, Serialize};
use wasmworker::{webworker_channel, worker_pool, Channel, WebWorker};
use wasmworker_proc_macro::webworker_channel_fn;

use crate::js_assert_eq;

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

/// Helper to wait for progress while keeping the task alive.
async fn wait_for_progress<F>(task: &mut F, channel: &Channel) -> Progress
where
    F: std::future::Future<Output = ProcessResult> + Unpin,
{
    let recv = channel.recv::<Progress>();
    pin_mut!(recv);

    match select(task, recv).await {
        Either::Left((result, _)) => {
            wasm_bindgen::throw_str(&format!(
                "Task completed unexpectedly before progress: {:?}",
                result
            ));
        }
        Either::Right((progress, _)) => progress.expect("Should receive progress"),
    }
}

/// Test that channel functions work with a single WebWorker.
pub(crate) async fn can_use_channel_with_worker() {
    let worker = WebWorker::new(None).await.expect("Couldn't create worker");

    // Create a channel for communication
    let (main_channel, worker_port) = Channel::new().expect("Couldn't create channel");

    // Prepare input data
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // Start the task (returns a future)
    let mut task = Box::pin(worker.run_channel(
        webworker_channel!(process_with_progress),
        &data,
        worker_port,
    ));

    // Wait for 50% progress
    let progress = wait_for_progress(&mut task, &main_channel).await;
    js_assert_eq!(progress.percent, 50, "Should be at 50%");

    // Tell the worker to continue
    main_channel.send(&Continue {
        should_continue: true,
    });

    // Wait for 100% progress
    let final_progress = wait_for_progress(&mut task, &main_channel).await;
    js_assert_eq!(final_progress.percent, 100, "Should be at 100%");

    // Now wait for the task to complete
    let result = task.await;
    js_assert_eq!(result.items_processed, 10, "Should process all items");
    js_assert_eq!(result.was_cancelled, false, "Should not be cancelled");
}

/// Test that channel functions work with cancellation.
pub(crate) async fn can_cancel_channel_task() {
    let worker = WebWorker::new(None).await.expect("Couldn't create worker");

    // Create a channel for communication
    let (main_channel, worker_port) = Channel::new().expect("Couldn't create channel");

    // Prepare input data
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // Start the task
    let mut task = Box::pin(worker.run_channel(
        webworker_channel!(process_with_progress),
        &data,
        worker_port,
    ));

    // Wait for 50% progress
    let progress = wait_for_progress(&mut task, &main_channel).await;
    js_assert_eq!(progress.percent, 50, "Should be at 50%");

    // Tell the worker to cancel
    main_channel.send(&Continue {
        should_continue: false,
    });

    // Wait for result (no 100% progress expected since we cancelled)
    let result = task.await;
    js_assert_eq!(result.items_processed, 5, "Should process half the items");
    js_assert_eq!(result.was_cancelled, true, "Should be cancelled");
}

/// Test that channel functions work with the worker pool.
pub(crate) async fn can_use_channel_with_pool() {
    let pool = worker_pool().await;

    // Create a channel for communication
    let (main_channel, worker_port) = Channel::new().expect("Couldn't create channel");

    // Prepare input data
    let data = vec![1, 2, 3, 4];

    // Start the task on the pool
    let mut task = Box::pin(pool.run_channel(
        webworker_channel!(process_with_progress),
        &data,
        worker_port,
    ));

    // Wait for 50% progress
    let progress = wait_for_progress(&mut task, &main_channel).await;
    js_assert_eq!(progress.percent, 50, "Should be at 50%");

    // Tell the worker to continue
    main_channel.send(&Continue {
        should_continue: true,
    });

    // Wait for 100% progress
    let final_progress = wait_for_progress(&mut task, &main_channel).await;
    js_assert_eq!(final_progress.percent, 100, "Should be at 100%");

    // Wait for completion
    let result = task.await;
    js_assert_eq!(result.items_processed, 4, "Should process all items");
    js_assert_eq!(result.was_cancelled, false, "Should not be cancelled");
}
