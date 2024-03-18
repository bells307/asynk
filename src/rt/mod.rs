pub mod builder;

mod handle;
mod task;

use self::{
    handle::JoinHandle,
    task::{BlockOnWaker, Task, TaskWaker},
};
use super::tp;
use crate::{tp::ThreadPool, AsyncRuntimeBuilder};
use futures::channel::oneshot;
use parking_lot::{Condvar, Mutex};
use std::{
    cell::RefCell,
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::Wake,
};

thread_local! {
    static RUNTIME: RefCell<Option<AsyncRuntime>> = RefCell::new(None);
}

/// Асинхронный рантайм поверх пула потоков
#[derive(Clone)]
pub struct AsyncRuntime {
    thread_pool: ThreadPool,
}

impl AsyncRuntime {
    pub fn builder() -> AsyncRuntimeBuilder {
        AsyncRuntimeBuilder::new()
    }

    fn new(thread_count: usize) -> Self {
        assert!(thread_count != 0);

        let thread_pool = ThreadPool::new(thread_count);
        Self { thread_pool }
    }

    /// Зарегистрировать рантайм на текущем потоке
    pub fn register(self) {
        RUNTIME.with(|e| {
            let mut ref_mut = e.borrow_mut();

            if ref_mut.is_some() {
                panic!("AsyncRuntime is already registered");
            };

            *ref_mut = Some(self)
        });
    }

    /// Заблокировать текущий поток до выполнения задачи
    pub fn block_on<F, Fut>(fut: F) -> Result<(), tp::JoinError>
    where
        F: Fn(Self) -> Fut,
        Fut: Future<Output = ()> + Send + 'static,
    {
        RUNTIME.with(|rt| {
            let rt = rt
                .borrow()
                .as_ref()
                .expect("runtime is not registered")
                .clone();

            let tp = rt.thread_pool.clone();

            let fut = Box::pin(fut(rt));

            let blocked = {
                let blocked_flag = Mutex::new(AtomicBool::new(true));
                let blocked_cv = Condvar::new();
                Arc::new((blocked_flag, blocked_cv))
            };

            let task = Task::new(
                Mutex::new(Some(fut)),
                tp.clone(),
                BlockOnWaker::new(blocked.clone()),
            );

            Arc::new(task).wake();

            let mut lock = blocked.0.lock();

            while lock.load(Ordering::Acquire) {
                blocked.1.wait(&mut lock);
            }

            tp.join()
        })
    }

    /// Создать новую асинхронную задачу
    pub fn spawn<T, F, Fut>(&self, fut: F) -> JoinHandle<T>
    where
        T: Send + 'static,
        F: Fn(Self) -> Fut,
        Fut: Future<Output = T> + Send + 'static,
    {
        let fut = Box::pin(fut(self.clone()));
        let (res_tx, res_rx) = oneshot::channel();

        let task = Task::new(
            Mutex::new(Some(fut)),
            self.thread_pool.clone(),
            TaskWaker::new(Mutex::new(Some(res_tx))),
        );

        Arc::new(task).wake();

        JoinHandle::new(res_rx)
    }
}
