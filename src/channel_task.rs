use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::oneshot;

use crate::{channel::Channel, convert::from_bytes};

/// A handle to a running channel task on a WebWorker.
///
/// `ChannelTask` combines a bidirectional [`Channel`] for sending and receiving
/// messages with the worker, and a future that resolves to the task's final result.
///
/// This type is returned by [`crate::WebWorker::run_channel`] and
/// [`crate::pool::WebWorkerPool::run_channel`]. It allows you to exchange messages
/// with the worker (e.g., for progress reporting) and then consume the final result.
///
/// # Example
///
/// ```ignore
/// let task = worker
///     .run_channel(webworker_channel!(process_with_progress), &data)
///     .await;
///
/// let progress: Progress = task.recv().await.expect("progress");
/// task.send(&Continue { should_continue: true });
///
/// let result: ProcessResult = task.result().await;
/// ```
pub struct ChannelTask<R> {
    channel: Channel,
    result_rx: oneshot::Receiver<Vec<u8>>,
    _phantom: PhantomData<R>,
}

impl<R: DeserializeOwned> ChannelTask<R> {
    /// Create a new `ChannelTask` from a channel and a result receiver.
    pub(crate) fn new(channel: Channel, result_rx: oneshot::Receiver<Vec<u8>>) -> Self {
        Self {
            channel,
            result_rx,
            _phantom: PhantomData,
        }
    }

    /// Receive the next deserialized message from the worker.
    ///
    /// Returns `None` if the channel's sender side has been dropped
    /// (i.e., the worker has finished and closed the channel).
    pub async fn recv<T: DeserializeOwned>(&self) -> Option<T> {
        self.channel.recv().await
    }

    /// Receive raw bytes from the worker.
    ///
    /// Returns `None` if the channel's sender side has been dropped.
    pub async fn recv_bytes(&self) -> Option<Box<[u8]>> {
        self.channel.recv_bytes().await
    }

    /// Send a serialized message to the worker.
    pub fn send<T: Serialize>(&self, msg: &T) {
        self.channel.send(msg);
    }

    /// Send raw bytes to the worker.
    pub fn send_bytes(&self, bytes: &[u8]) {
        self.channel.send_bytes(bytes);
    }

    /// Await the task's final result, consuming the `ChannelTask`.
    pub async fn result(self) -> R {
        let bytes = self
            .result_rx
            .await
            .expect("WebWorker result sender dropped");
        from_bytes(&bytes)
    }
}
