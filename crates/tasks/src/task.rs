use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_lite::Future;

pub struct Task<T>(async_executor::Task<T>);

impl<T> Task<T> {
    pub(crate) fn new(task: async_executor::Task<T>) -> Self {
        Self(task)
    }

    pub fn detach(self) {
        self.0.detach()
    }
}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx)
    }
}
