use std::marker::PhantomData;

use futures::future::LocalBoxFuture;

use crate::Channel;

/// This struct describes a simple synchronous function to be called by the worker.
/// It ensures type safety when constructed using the [`crate::webworker!`] macro.
///
/// For async functions with channel support, use [`WebWorkerChannelFn`] instead.
pub struct WebWorkerFn<T, R> {
    /// The name of the original function.
    /// The worker will automatically add the `__webworker_` prefix.
    pub(crate) name: &'static str,
    /// The original function, which can be used as a fallback.
    #[cfg_attr(not(feature = "iter-ext"), allow(dead_code))]
    pub(crate) func: fn(T) -> R,
}

impl<T, R> Clone for WebWorkerFn<T, R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, R> Copy for WebWorkerFn<T, R> {}

impl<T, R> WebWorkerFn<T, R> {
    /// Manually creates a [`WebWorkerFn`] object.
    /// This function should be avoided in most cases as it does not guarantee that the function
    /// has the right type or is exposed to the worker.
    ///
    /// Instead use the [`crate::webworker!`] macro to create an instance of this type.
    pub fn new_unchecked(func_name: &'static str, f: fn(T) -> R) -> Self {
        Self {
            name: func_name,
            func: f,
        }
    }
}

/// This struct describes an async function with channel support to be called by the worker.
/// It ensures type safety when constructed using the [`crate::webworker_channel!`] macro.
///
/// The channel allows bidirectional communication between the worker and the main thread
/// during function execution, enabling use cases like progress reporting and interactive workflows.
///
/// For simple synchronous functions, use [`WebWorkerFn`] instead.
pub struct WebWorkerChannelFn<T, R> {
    /// The name of the original function.
    /// The worker will automatically add the `__webworker_channel_` prefix.
    pub(crate) name: &'static str,
    /// The original function, which can be used as a fallback.
    /// Currently unused but kept for future fallback functionality.
    #[allow(dead_code)]
    pub(crate) func: fn(T, Channel) -> LocalBoxFuture<'static, R>,
    /// Phantom data for the input type (needed since T isn't used directly in fields).
    pub(crate) _phantom: PhantomData<fn(T) -> R>,
}

impl<T, R> Clone for WebWorkerChannelFn<T, R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, R> Copy for WebWorkerChannelFn<T, R> {}

impl<T, R> WebWorkerChannelFn<T, R> {
    /// Manually creates a [`WebWorkerChannelFn`] object.
    /// This function should be avoided in most cases as it does not guarantee that the function
    /// has the right type or is exposed to the worker.
    ///
    /// Instead use the [`crate::webworker_channel!`] macro to create an instance of this type.
    pub fn new_unchecked(
        func_name: &'static str,
        f: fn(T, Channel) -> LocalBoxFuture<'static, R>,
    ) -> Self {
        Self {
            name: func_name,
            func: f,
            _phantom: PhantomData,
        }
    }
}

/// This macro safely instantiates a [`WebWorkerFn`] instance to be passed to a [`crate::WebWorker`].
/// It ensures that the function is exposed via the `#[webworker_fn]` procedural macro.
///
/// Example:
/// ```no_run
/// # use serde::{Serialize, Deserialize};
/// # use wasmworker_proc_macro::webworker_fn;
/// # use wasmworker::{webworker, func::WebWorkerFn};
/// # #[derive(Serialize, Deserialize)]
/// # struct VecType(Vec<u32>);
/// #[webworker_fn]
/// pub fn sort_vec(mut v: VecType) -> VecType {
///     v.0.sort();
///     v
/// }
///
/// # fn main() {
/// let func: WebWorkerFn<VecType, VecType> = webworker!(sort_vec);
/// # }
/// ```
#[macro_export]
macro_rules! webworker {
    ($name:ident) => {{
        let _ = $name::__WEBWORKER;
        $crate::func::WebWorkerFn::new_unchecked(stringify!($name), $name)
    }};
}

/// This macro safely instantiates a [`WebWorkerChannelFn`] instance to be passed to a [`crate::WebWorker`].
/// It ensures that the function is exposed via the `#[webworker_channel_fn]` procedural macro.
///
/// Example:
/// ```ignore
/// #[webworker_channel_fn]
/// pub async fn process_with_progress(data: Vec<u8>, channel: Channel) -> Result<Output, Error> {
///     channel.send(&Progress { percent: 50 });
///     let response: UserChoice = channel.recv().await?;
///     // ... process data ...
///     Ok(output)
/// }
///
/// let func: WebWorkerChannelFn<Vec<u8>, Result<Output, Error>> = webworker_channel!(process_with_progress);
/// ```
#[macro_export]
macro_rules! webworker_channel {
    ($name:ident) => {{
        let _ = $name::__WEBWORKER_CHANNEL;
        $crate::func::WebWorkerChannelFn::new_unchecked(stringify!($name), |a, c| {
            Box::pin($name(a, c))
        })
    }};
}
