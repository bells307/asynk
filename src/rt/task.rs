use super::AsyncRuntime;
use parking_lot::Mutex;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Wake},
};

type TaskFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub(crate) struct Task<T> {
    fut: Mutex<Option<TaskFuture<T>>>,
    rt: AsyncRuntime,
    ready_fn: Box<dyn Fn(T) + Send + Sync + 'static>,
}

impl<T> Task<T>
where
    T: Send + 'static,
{
    pub(crate) fn new(
        fut: TaskFuture<T>,
        rt: AsyncRuntime,
        ready_fn: impl Fn(T) + Send + Sync + 'static,
    ) -> Self {
        Self {
            fut: Mutex::new(Some(fut)),
            rt,
            ready_fn: Box::new(ready_fn),
        }
    }

    pub(crate) fn poll(self: Arc<Self>) {
        let mut lock = self.fut.lock();
        if let Some(mut fut) = lock.take() {
            let waker = Arc::clone(&self).into();
            let mut cx = Context::from_waker(&waker);
            match fut.as_mut().poll(&mut cx) {
                Poll::Ready(res) => (self.ready_fn)(res),
                Poll::Pending => *lock = Some(fut),
            };
        };
    }
}

impl<T> Wake for Task<T>
where
    T: Send + 'static,
{
    fn wake(self: Arc<Self>) {
        self.rt.schedule_task(Arc::clone(&self))
    }
}
