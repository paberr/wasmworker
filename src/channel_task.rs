use std::marker::PhantomData;

use futures::{
    future::{select, Either},
    pin_mut,
};
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::{oneshot, watch};

use crate::{channel::Channel, convert::from_bytes, error::TaskError};

/// A handle to a running channel task on a WebWorker.
///
/// `ChannelTask` combines a bidirectional [`Channel`] for sending and receiving
/// messages with the worker, and a future that resolves to the task's final result.
///
/// This type is returned by [`crate::WebWorker::run_channel`] and
/// [`crate::pool::WebWorkerPool::run_channel`]. It allows you to exchange messages
/// with the worker (e.g., for progress reporting) and then consume the final result.
///
/// If the worker is terminated while the task is still running, [`ChannelTask::recv`]
/// returns `None` and [`ChannelTask::result`] returns [`TaskError::WorkerTerminated`].
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
/// let result: ProcessResult = task.result().await.expect("worker terminated");
/// ```
pub struct ChannelTask<R> {
    channel: Channel,
    result_rx: oneshot::Receiver<Vec<u8>>,
    /// Set to `true` once the worker running this task has been terminated.
    terminated: watch::Receiver<bool>,
    _phantom: PhantomData<R>,
}

impl<R: DeserializeOwned> ChannelTask<R> {
    /// Create a new `ChannelTask` from a channel, a result receiver, and the
    /// termination signal of the worker running the task.
    pub(crate) fn new(
        channel: Channel,
        result_rx: oneshot::Receiver<Vec<u8>>,
        terminated: watch::Receiver<bool>,
    ) -> Self {
        Self {
            channel,
            result_rx,
            terminated,
            _phantom: PhantomData,
        }
    }

    /// Receive the next deserialized message from the worker.
    ///
    /// Returns `None` once the worker has been terminated and all messages it
    /// already sent have been received.
    pub async fn recv<T: DeserializeOwned>(&self) -> Option<T> {
        let bytes = self.recv_bytes().await?;
        Some(from_bytes(&bytes))
    }

    /// Receive raw bytes from the worker.
    ///
    /// Returns `None` once the worker has been terminated and all messages it
    /// already sent have been received.
    pub async fn recv_bytes(&self) -> Option<Box<[u8]>> {
        // Messages that already arrived are handed out before reporting the
        // termination, so no message is lost when a worker is terminated.
        let mut terminated = self.terminated.clone();
        let message = self.channel.recv_bytes();
        let terminated = terminated.changed();
        pin_mut!(message, terminated);

        match select(message, terminated).await {
            Either::Left((message, _)) => message,
            Either::Right(_) => None,
        }
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
    ///
    /// Returns [`TaskError::WorkerTerminated`] if the worker was terminated
    /// before the task returned a result.
    pub async fn result(self) -> Result<R, TaskError> {
        let bytes = self
            .result_rx
            .await
            .map_err(|_| TaskError::WorkerTerminated)?;
        Ok(from_bytes(&bytes))
    }
}
