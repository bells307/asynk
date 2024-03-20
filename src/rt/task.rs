use futures::channel::oneshot;
use parking_lot::Mutex;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Wake},
};

use super::AsyncRuntime;

type TaskFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;
type MutexOpt<T> = Mutex<Option<T>>;

pub(crate) struct Task<T> {
    fut: MutexOpt<TaskFuture<T>>,
    res_tx: MutexOpt<oneshot::Sender<T>>,
    rt: AsyncRuntime,
}

impl<T> Task<T>
where
    T: Send + 'static,
{
    pub(crate) fn new(fut: TaskFuture<T>, res_tx: oneshot::Sender<T>, rt: AsyncRuntime) -> Self {
        Self {
            fut: Mutex::new(Some(fut)),
            res_tx: Mutex::new(Some(res_tx)),
            rt,
        }
    }

    pub(crate) fn poll(self: Arc<Self>) {
        let mut lock = self.fut.lock();
        if let Some(mut fut) = lock.take() {
            let waker = Arc::clone(&self).into();
            let mut cx = Context::from_waker(&waker);
            match fut.as_mut().poll(&mut cx) {
                Poll::Ready(res) => {
                    self.res_tx
                        .lock()
                        .take()
                        .expect("task result channel is empty")
                        .send(res)
                        .map_err(|_| ())
                        // Here we need to be carefully, `JoinHandle` owns the sender, so if it'll drop
                        // earlier than task will complete, so it will cause panic.
                        .expect("task result channel is dropped");
                }
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
