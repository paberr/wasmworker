use std::borrow::Borrow;

use futures::future::join_all;
use serde::{Deserialize, Serialize};

use crate::{func::WebWorkerFn, global::has_worker_pool, worker_pool};

/// This extension trait defines the method [`IteratorExt::par_map`],
/// which will use the default [`crate::pool::WebWorkerPool`] as returned by [`worker_pool()`].
pub trait IteratorExt<T>: Sized + Iterator
where
    Self::Item: Borrow<T>,
    T: Serialize + for<'de> Deserialize<'de>,
{
    /// The `par_map` function allows to parallelize a map operation on the default
    /// [`crate::pool::WebWorkerPool`] as returned by [`worker_pool()`].
    ///
    /// For each element of the iterator, a new task is scheduled on the worker pool.
    /// Only functions that are annotated with the `#[webworker_fn]` macro can be used.
    ///
    /// Example:
    /// ```ignore
    /// #[webworker_fn]
    /// fn my_func(arg: T) -> R { /*...*/ }
    ///
    /// let vec = vec![ /*...*/ ];
    /// vec.iter().par_map(webworker!(my_func)).await
    /// ```
    #[allow(async_fn_in_trait)]
    async fn par_map<R>(self, func: WebWorkerFn<T, R>) -> Vec<R>
    where
        R: Serialize + for<'de> Deserialize<'de>,
    {
        let pool = worker_pool().await;
        join_all(self.map(|arg| pool.run_internal(func, arg))).await
    }

    /// The `try_par_map` function will attempt to parallelize a map operation on the default
    /// [`crate::pool::WebWorkerPool`] if it has previously been initialized by [`init_worker_pool()`].
    ///
    /// If the worker pool has been initialized, the function will behave exactly like `IteratorExt::par_map`.
    /// If the worker pool has not been initialized, the function will behave like a regular map operation
    /// on the main thread.
    ///
    /// Example:
    /// ```ignore
    /// #[webworker_fn]
    /// fn my_func(arg: T) -> R { /*...*/ }
    ///
    /// let vec = vec![ /*...*/ ];
    /// vec.iter().try_par_map(webworker!(my_func)).await
    /// ```
    #[allow(async_fn_in_trait)]
    async fn try_par_map<R>(self, func: WebWorkerFn<T, R>) -> Vec<R>
    where
        Self::Item: Into<T>,
        R: Serialize + for<'de> Deserialize<'de>,
    {
        if has_worker_pool() {
            self.par_map(func).await
        } else {
            self.map(|item| (func.func)(item.into())).collect()
        }
    }
}

impl<T, I> IteratorExt<T> for I
where
    I: Iterator,
    I::Item: Borrow<T>,
    T: Serialize + for<'de> Deserialize<'de>,
{
}
