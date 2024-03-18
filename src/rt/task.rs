use crate::tp::ThreadPool;
use futures::channel::oneshot;
use parking_lot::{Condvar, Mutex};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll, Wake},
};

type TaskFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

pub(crate) struct Task<T, W> {
    fut: Mutex<Option<TaskFuture<T>>>,
    thread_pool: ThreadPool,
    waker: W,
}

impl<T, W> Task<T, W> {
    pub(crate) fn new(
        fut: Mutex<Option<TaskFuture<T>>>,
        thread_pool: ThreadPool,
        waker: W,
    ) -> Self {
        Self {
            fut,
            thread_pool,
            waker,
        }
    }
}

pub(crate) struct BlockOnWaker {
    blocked: Arc<(Mutex<AtomicBool>, Condvar)>,
}

impl BlockOnWaker {
    pub(crate) fn new(blocked: Arc<(Mutex<AtomicBool>, Condvar)>) -> Self {
        Self { blocked }
    }
}

impl Wake for Task<(), BlockOnWaker> {
    fn wake(self: Arc<Self>) {
        let task = Arc::clone(&self);
        self.thread_pool.spawn(move || {
            let mut lock = task.fut.lock();
            if let Some(mut fut) = lock.take() {
                let waker = Arc::clone(&task).into();
                let mut cx = Context::from_waker(&waker);
                match fut.as_mut().poll(&mut cx) {
                    Poll::Ready(_) => {
                        task.waker.blocked.0.lock().store(false, Ordering::Release);
                        task.waker.blocked.1.notify_one();
                    }
                    Poll::Pending => *lock = Some(fut),
                };
            };
        });
    }
}

pub(crate) struct TaskWaker<T> {
    res_tx: Mutex<Option<oneshot::Sender<T>>>,
}

impl<T> TaskWaker<T> {
    pub(crate) fn new(res_tx: Mutex<Option<oneshot::Sender<T>>>) -> Self {
        Self { res_tx }
    }
}

impl<T> Wake for Task<T, TaskWaker<T>>
where
    T: Send + 'static,
{
    fn wake(self: Arc<Self>) {
        let task = Arc::clone(&self);
        self.thread_pool.spawn(move || {
            let mut lock = task.fut.lock();
            if let Some(mut fut) = lock.take() {
                let waker = Arc::clone(&task).into();
                let mut cx = Context::from_waker(&waker);
                match fut.as_mut().poll(&mut cx) {
                    Poll::Ready(res) => {
                        task.waker
                            .res_tx
                            .lock()
                            .take()
                            .expect("task result channel is empty")
                            .send(res)
                            .map_err(|_| ())
                            .expect("task result channel is dropped");
                    }
                    Poll::Pending => *lock = Some(fut),
                };
            };
        });
    }
}
