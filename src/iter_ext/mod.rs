use std::borrow::Borrow;

use futures::future::join_all;
use serde::{Deserialize, Serialize};

use crate::{func::WebWorkerFn, worker_pool};

pub trait IteratorExt<T>: Sized + Iterator
where
    Self::Item: Borrow<T>,
    T: Serialize + for<'de> Deserialize<'de>,
{
    #[allow(async_fn_in_trait)]
    async fn par_map<R>(self, func: WebWorkerFn<T, R>) -> Vec<R>
    where
        R: Serialize + for<'de> Deserialize<'de>,
    {
        let pool = worker_pool().await;
        join_all(self.map(|arg| pool.run_internal(func, arg))).await
    }
}

impl<T, I> IteratorExt<T> for I
where
    I: Iterator,
    I::Item: Borrow<T>,
    T: Serialize + for<'de> Deserialize<'de>,
{
}
