use serde::{Deserialize, Serialize};

/// Message sent by the worker after initialization.
/// This is used to alert the main thread that initialization is complete.
/// It also indicates if errors occurred during the import.
#[derive(Deserialize)]
pub(super) struct PostInit {
    /// `true` if initialization is complete, and `false` if import errors occurred.
    pub(crate) success: bool,
    /// The `message` is only set if `success` is false.
    /// It contains a description of the error that occurred.
    #[serde(default)]
    pub(crate) message: Option<String>,
}

/// This message is sent to the worker when a new task should be executed.
#[derive(Serialize, Deserialize)]
pub(super) struct Request {
    /// This is the internal task id, which is used to match a [`Response`]
    /// to the corresponding task.
    pub(crate) id: usize,
    /// The name of the function to be executed by the worker.
    pub(crate) func_name: &'static str,
    /// The serialized argument to be passed to the function.
    /// Serialization is done using [`crate::convert::to_bytes`].
    #[serde(with = "serde_bytes")]
    pub(crate) arg: Box<[u8]>,
}

/// This message is sent back from the worker once a task is completed,
/// i.e., the function has been executed successfully and we have a result.
#[derive(Serialize, Deserialize)]
pub(super) struct Response {
    /// The corresponding task id, matching the original id from the [`Request`] object.
    pub(crate) id: usize,
    /// The response, which should only be `None` if the function could not be found.
    /// This should never be the case if the [`crate::func::WebWorkerFn`] was constructed
    /// using the [`crate::webworker!`] macro.
    #[serde(with = "serde_bytes")]
    pub(crate) response: Option<Vec<u8>>,
}
