use crate::AsyncRuntime;
use futures::channel::oneshot;
use parking_lot::Mutex;
use std::{future::Future, pin::Pin, sync::Arc, task::Wake};

type TaskFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub(crate) struct Task<T> {
    pub(super) fut: Mutex<Option<TaskFuture<T>>>,
    pub(super) res_tx: Mutex<Option<oneshot::Sender<T>>>,
    rt: AsyncRuntime,
}

impl<T> Task<T> {
    pub(crate) fn new(fut: TaskFuture<T>, res_tx: oneshot::Sender<T>, rt: AsyncRuntime) -> Self {
        Self {
            fut: Mutex::new(Some(fut)),
            res_tx: Mutex::new(Some(res_tx)),
            rt,
        }
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
