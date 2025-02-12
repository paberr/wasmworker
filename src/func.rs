/// This struct describes the function to be called by the worker.
/// It also ensures type safety, when constructed using the [`crate::webworker!`] macro.
pub struct WebWorkerFn<T, R> {
    /// The name of the original function.
    /// The worker will automatically add the `__webworker_` prefix.
    pub(crate) name: &'static str,
    /// The original function, which can be used as a fallback.
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
    /// This function should be avoided in most cases as it does guarantee that the function
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

/// This macro safely instantiates a [`WebWorkerFn`] instance to be passed to a [`crate::WebWorker`].
/// It ensures that the function is exposed via the `#[webworker_fn]` procedural macro.
///
/// Example:
/// ```ignore
/// #[webworker_fn]
/// pub fn sort_vec(mut v: VecType) -> VecType {
///     v.0.sort();
///     v
/// }
///
/// let func: WebWorkerFn<VecType, VecType> = webworker!(sort_vec);
/// ```
#[macro_export]
macro_rules! webworker {
    ($name:ident) => {{
        let _ = $name::__WEBWORKER;
        $crate::func::WebWorkerFn::new_unchecked(stringify!($name), $name)
    }};
}
