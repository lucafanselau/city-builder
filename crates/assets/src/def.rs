use std::pin::Pin;
use tasks::futures::Future;

pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
