use futures::{channel::oneshot, FutureExt};
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};

use super::task::Task;

#[derive(Debug, thiserror::Error)]
#[error("join fail: result channel dropped")]
pub struct JoinError;

pub struct JoinHandle<T> {
    rx: oneshot::Receiver<T>,
    task: Arc<Task<T>>,
}

impl<T> JoinHandle<T>
where
    T: Send + 'static,
{
    pub(crate) fn new(rx: oneshot::Receiver<T>, task: Arc<Task<T>>) -> Self {
        Self { rx, task }
    }

    pub(crate) fn waker(&self) -> Waker {
        self.task.clone().into()
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.rx.poll_unpin(cx).map_err(|_| JoinError)
    }
}
