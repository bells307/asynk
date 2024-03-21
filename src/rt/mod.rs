pub mod builder;
pub mod handle;

mod task;

use self::{handle::JoinHandle, task::Task};
use crate::{reactor::Reactor, tp::ThreadPool};
use futures::channel::oneshot;
use parking_lot::Mutex;
use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

/// Asynchronous runtime on top of the thread pool
#[derive(Clone)]
pub struct AsyncRuntime(Arc<Inner>);

struct Inner {
    thread_pool: ThreadPool,
    reactor: Reactor,
}

impl AsyncRuntime {
    pub(crate) fn new(thread_count: usize) -> Self {
        assert!(thread_count != 0);

        let thread_pool = ThreadPool::new(thread_count);
        let reactor = Reactor::new();

        Self(Arc::new(Inner {
            thread_pool,
            reactor,
        }))
    }

    /// Block current thread until main task completion
    pub(crate) fn block_on<F>(&self, fut: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let completed = Arc::new(AtomicBool::new(false));

        // When the main task becomes `Ready`, than we set complete flag as true
        let ready_fn = {
            let completed = Arc::clone(&completed);
            move |_| {
                completed.store(true, Ordering::Release);
            }
        };

        self.spawn(fut, ready_fn);

        loop {
            if completed.load(Ordering::Acquire) {
                break;
            } else {
                //  While the main thread has nothing to do, let’s give it to read reactor events
                self.0.reactor.poll_events().unwrap();
            }
        }

        self.0
            .thread_pool
            .join()
            .expect("runtime thread pool join error");
    }

    /// Create new async task
    pub(crate) fn spawn_task<T, F>(&self, fut: F) -> JoinHandle<T>
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        let (res_tx, res_rx) = oneshot::channel();

        let res_tx = Mutex::new(Some(res_tx));
        let ready_fn = move |res| {
            res_tx
                .lock()
                .take()
                .expect("task result channel is empty")
                .send(res)
                // If sending caused an error, it means that `JoinHandle` was dropped before
                // await'ing him. That's okay, ignore this error.
                .ok();
        };

        self.spawn(fut, ready_fn);

        JoinHandle::new(res_rx)
    }

    fn spawn<T, F>(&self, fut: F, ready_fn: impl Fn(T) + Send + Sync + 'static)
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        let fut = Box::pin(fut);

        let task = Task::new(fut, self.clone(), ready_fn).into();

        // Immediately ask the task to begin execution
        self.schedule_task(task);
    }

    pub(crate) fn reactor(&self) -> &Reactor {
        &self.0.reactor
    }

    /// Schedule task for polling
    fn schedule_task<T>(&self, task: Arc<Task<T>>)
    where
        T: Send + 'static,
    {
        self.0.thread_pool.spawn(move || Arc::clone(&task).poll());
    }
}
