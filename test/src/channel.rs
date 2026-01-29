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

            // Wait for confirmation to continue (must not be None — that
            // would mean the channel was closed before a response was sent).
            let cont: Continue = channel
                .recv()
                .await
                .expect("Channel closed unexpectedly before receiving Continue");
            if !cont.should_continue {
                return ProcessResult {
                    items_processed: processed,
                    was_cancelled: true,
                };
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

    // Prepare input data
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // Start the task — run_channel creates the channel internally and returns it
    // alongside a future for the result. Both travel through the same MessagePort,
    // so channel messages are guaranteed to arrive before the result (FIFO).
    let (channel, result) = worker
        .run_channel(webworker_channel!(process_with_progress), &data)
        .await;

    // Wait for 50% progress
    let progress: Progress = channel
        .recv()
        .await
        .expect("Should receive 50% progress");
    js_assert_eq!(progress.percent, 50, "Should be at 50%");

    // Tell the worker to continue
    channel.send(&Continue {
        should_continue: true,
    });

    // Wait for 100% progress
    let final_progress: Progress = channel
        .recv()
        .await
        .expect("Should receive 100% progress");
    js_assert_eq!(final_progress.percent, 100, "Should be at 100%");

    // Wait for the task to complete
    let result = result.await;
    js_assert_eq!(result.items_processed, 10, "Should process all items");
    js_assert_eq!(result.was_cancelled, false, "Should not be cancelled");
}

/// Test that channel functions work with cancellation.
pub(crate) async fn can_cancel_channel_task() {
    let worker = WebWorker::new(None).await.expect("Couldn't create worker");

    // Prepare input data
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // Start the task
    let (channel, result) = worker
        .run_channel(webworker_channel!(process_with_progress), &data)
        .await;

    // Wait for 50% progress
    let progress: Progress = channel
        .recv()
        .await
        .expect("Should receive 50% progress");
    js_assert_eq!(progress.percent, 50, "Should be at 50%");

    // Tell the worker to cancel
    channel.send(&Continue {
        should_continue: false,
    });

    // Wait for result (no 100% progress expected since we cancelled)
    let result = result.await;
    js_assert_eq!(result.items_processed, 5, "Should process half the items");
    js_assert_eq!(result.was_cancelled, true, "Should be cancelled");
}

/// Test that channel functions work with the worker pool.
pub(crate) async fn can_use_channel_with_pool() {
    let pool = worker_pool().await;

    // Prepare input data
    let data = vec![1, 2, 3, 4];

    // Start the task on the pool
    let (channel, result) = pool
        .run_channel(webworker_channel!(process_with_progress), &data)
        .await;

    // Wait for 50% progress
    let progress: Progress = channel
        .recv()
        .await
        .expect("Should receive 50% progress");
    js_assert_eq!(progress.percent, 50, "Should be at 50%");

    // Tell the worker to continue
    channel.send(&Continue {
        should_continue: true,
    });

    // Wait for 100% progress
    let final_progress: Progress = channel
        .recv()
        .await
        .expect("Should receive 100% progress");
    js_assert_eq!(final_progress.percent, 100, "Should be at 100%");

    // Wait for completion
    let result = result.await;
    js_assert_eq!(result.items_processed, 4, "Should process all items");
    js_assert_eq!(result.was_cancelled, false, "Should not be cancelled");
}
