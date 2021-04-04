//! Contains some quality of life functions for working with futures

use core::anyhow::Result;
use futures_lite::{stream, Future, StreamExt};

// Implementation is based on https://github.com/smol-rs/futures-lite/issues/2#issuecomment-696141698
pub fn join_all<'a, T: 'a>(
    iter: impl Iterator<Item = impl Future<Output = T> + 'a> + 'a,
) -> impl Future<Output = Vec<T>> + 'a {
    stream::iter(iter).then(|f| f).collect()
}

pub async fn try_join_all<'a, T>(
    iter: impl Iterator<Item = impl Future<Output = Result<T>> + 'a> + 'a,
) -> Result<Vec<T>> {
    join_all(iter).await.into_iter().collect()
}
