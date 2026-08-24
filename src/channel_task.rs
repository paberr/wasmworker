use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    rc::Rc,
};

use futures::{future::select, pin_mut};
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::{oneshot, watch};

use crate::{channel::Channel, convert::from_bytes, error::TaskError};

type LifecycleCallback = Box<dyn FnOnce()>;

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
/// let result: ProcessResult = task.result().await.expect("worker terminated");
/// ```
pub struct ChannelTask<R> {
    channel: Channel,
    result_rx: Option<oneshot::Receiver<Vec<u8>>>,
    control: ChannelTaskControl,
    on_complete: Option<LifecycleCallback>,
    _phantom: PhantomData<R>,
}

/// A cloneable handle for terminating a running [`ChannelTask`].
#[derive(Clone)]
pub struct ChannelTaskControl {
    inner: Rc<ChannelTaskControlInner>,
}

struct ChannelTaskControlInner {
    terminated: Cell<bool>,
    on_terminate: RefCell<Option<LifecycleCallback>>,
    close_tx: watch::Sender<bool>,
}

impl ChannelTaskControl {
    fn new(on_terminate: Option<LifecycleCallback>) -> Self {
        let (close_tx, _) = watch::channel(false);
        Self {
            inner: Rc::new(ChannelTaskControlInner {
                terminated: Cell::new(false),
                on_terminate: RefCell::new(on_terminate),
                close_tx,
            }),
        }
    }

    /// Terminate the worker running the associated channel task.
    ///
    /// Repeated calls are harmless.
    pub fn terminate(&self) {
        if self.inner.terminated.replace(true) {
            return;
        }

        let _ = self.inner.close_tx.send(true);
        if let Some(callback) = self.inner.on_terminate.borrow_mut().take() {
            callback();
        }
    }

    fn subscribe(&self) -> watch::Receiver<bool> {
        self.inner.close_tx.subscribe()
    }

    fn is_terminated(&self) -> bool {
        self.inner.terminated.get()
    }

    fn is_armed(&self) -> bool {
        self.inner.on_terminate.borrow().is_some()
    }

    fn disarm(&self) {
        self.inner.on_terminate.borrow_mut().take();
    }

    fn set_on_terminate(&self, callback: LifecycleCallback) {
        *self.inner.on_terminate.borrow_mut() = Some(callback);
    }
}

impl<R: DeserializeOwned> ChannelTask<R> {
    /// Create a new `ChannelTask` from a channel and a result receiver.
    #[doc(hidden)]
    pub fn new(channel: Channel, result_rx: oneshot::Receiver<Vec<u8>>) -> Self {
        Self::with_lifecycle(channel, result_rx, None, None)
    }

    #[doc(hidden)]
    pub(crate) fn with_lifecycle(
        channel: Channel,
        result_rx: oneshot::Receiver<Vec<u8>>,
        on_complete: Option<LifecycleCallback>,
        on_terminate: Option<LifecycleCallback>,
    ) -> Self {
        Self {
            channel,
            result_rx: Some(result_rx),
            control: ChannelTaskControl::new(on_terminate),
            on_complete,
            _phantom: PhantomData,
        }
    }

    pub(crate) fn with_callbacks(
        mut self,
        on_complete: LifecycleCallback,
        on_terminate: LifecycleCallback,
    ) -> Self {
        self.on_complete = Some(on_complete);
        self.control.set_on_terminate(on_terminate);
        self
    }

    /// Return a cloneable controller that can terminate this task externally.
    pub fn control(&self) -> ChannelTaskControl {
        self.control.clone()
    }

    /// Receive the next deserialized message from the worker.
    ///
    /// Returns `None` if the channel closes or the task is terminated.
    pub async fn recv<T: DeserializeOwned>(&self) -> Option<T> {
        let bytes = self.recv_bytes().await?;
        Some(from_bytes(&bytes))
    }

    /// Receive raw bytes from the worker.
    ///
    /// Returns `None` if the channel closes or the task is terminated.
    pub async fn recv_bytes(&self) -> Option<Box<[u8]>> {
        if self.control.is_terminated() {
            return None;
        }

        let mut close_rx = self.control.subscribe();
        let message = self.channel.recv_bytes();
        let closed = close_rx.changed();
        pin_mut!(message, closed);

        match select(message, closed).await {
            futures::future::Either::Left((message, _)) if !self.control.is_terminated() => message,
            _ => None,
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
    pub async fn result(mut self) -> Result<R, TaskError> {
        let result_rx = self
            .result_rx
            .take()
            .ok_or(TaskError::ResultAlreadyConsumed)?;
        let result = result_rx.await.map_err(|_| TaskError::WorkerTerminated);

        match result {
            Ok(bytes) if !self.control.is_terminated() => {
                self.control.disarm();
                if let Some(on_complete) = self.on_complete.take() {
                    on_complete();
                }
                Ok(from_bytes(&bytes))
            }
            Ok(_) | Err(_) => {
                self.control.terminate();
                Err(TaskError::WorkerTerminated)
            }
        }
    }

    /// Terminate the worker running this task.
    ///
    /// Pool tasks exclusively lease their worker. The pool replaces the terminated
    /// worker in the same slot before making that slot schedulable again.
    pub fn terminate(&self) {
        self.control.terminate();
    }
}

impl<R> Drop for ChannelTask<R> {
    fn drop(&mut self) {
        if self.control.is_armed() {
            self.control.terminate();
        }
    }
}
