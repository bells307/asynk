pub mod builder;

mod handle;
mod task;

use self::{handle::JoinHandle, task::Task};
use super::tp;
use crate::{tp::ThreadPool, AsyncRuntimeBuilder};
use futures::{channel::oneshot, FutureExt};
use std::{
    cell::RefCell,
    future::Future,
    sync::Arc,
    task::{Context, Poll, Wake},
    thread,
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

            let mut jh = rt.spawn(fut);

            loop {
                let waker = jh.waker();
                let mut cx = Context::from_waker(&waker);

                match jh.poll_unpin(&mut cx) {
                    Poll::Ready(res) => {
                        // Ошибка возможна в случае, если JoinHandle был дропнут ранее, но
                        // сейчас мы должны быть уверены, что он у нас в скоупе
                        res.expect("unexpected drop blocked task JoinHandle");
                        break;
                    }
                    Poll::Pending => {
                        // Временная заглушка в виде yield_now(). В дальнейшем, в случае, если
                        // основной таск еще не готов, то планируется ненадолго отдавать
                        // этот поток для обработки событий реактора
                        thread::yield_now()
                    }
                }
            }

            rt.thread_pool.join()
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

        let task = Task::new(fut, res_tx, self.clone()).into();

        // Сразу же просим задачу начать исполнение
        Arc::clone(&task).wake();

        JoinHandle::new(res_rx, task)
    }

    /// Запланировать задачу на выполнение
    fn schedule_task<T>(&self, task: Arc<Task<T>>)
    where
        T: Send + 'static,
    {
        self.thread_pool.spawn(move || Arc::clone(&task).poll());
    }
}
