pub mod builder;
pub mod handle;

mod task;

use self::{handle::JoinHandle, task::Task};
use crate::{reactor::Reactor, tp::ThreadPool};
use futures::{channel::oneshot, FutureExt};
use std::{
    future::Future,
    sync::Arc,
    task::{Context, Poll},
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
        let mut jh = self.spawn(fut);

        loop {
            let waker = jh.waker();
            let mut cx = Context::from_waker(&waker);

            match jh.poll_unpin(&mut cx) {
                Poll::Ready(res) => {
                    // The error is possible only when JoinHandle dropped earlier, but now
                    // we must be sure that we have the handle in scope
                    res.expect("unexpected drop blocked task JoinHandle");
                    break;
                }
                Poll::Pending => {
                    // While the main thread has nothing to do, let’s give it to read reactor events
                    self.0.reactor.poll_events().unwrap();
                }
            }
        }

        self.0
            .thread_pool
            .join()
            .expect("runtime thread pool join error");
    }

    /// Create new async task
    pub(crate) fn spawn<T, F>(&self, fut: F) -> JoinHandle<T>
    where
        T: Send + 'static,
        F: Future<Output = T> + Send + 'static,
    {
        let fut = Box::pin(fut);
        let (res_tx, res_rx) = oneshot::channel();

        let task = Task::new(fut, res_tx, self.clone()).into();

        // Immediately ask the task to begin execution
        self.schedule_task(Arc::clone(&task));

        JoinHandle::new(res_rx, task)
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
