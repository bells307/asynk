pub mod builder;

mod handle;
mod task;

use self::{handle::JoinHandle, task::Task};
use super::tp;
use crate::{reactor::Reactor, tp::ThreadPool, AsyncRuntimeBuilder};
use futures::{channel::oneshot, FutureExt};
use parking_lot::Mutex;
use std::{
    cell::RefCell,
    future::Future,
    sync::Arc,
    task::{Context, Poll},
};

thread_local! {
    static RUNTIME: RefCell<Mutex<Option<AsyncRuntime>>> = RefCell::new(Mutex::new(None));
}

/// Асинхронный рантайм поверх пула потоков
#[derive(Clone)]
pub struct AsyncRuntime(Arc<Inner>);

struct Inner {
    thread_pool: ThreadPool,
    reactor: Reactor,
}

impl AsyncRuntime {
    pub fn builder() -> AsyncRuntimeBuilder {
        AsyncRuntimeBuilder::new()
    }

    fn new(thread_count: usize) -> Self {
        assert!(thread_count != 0);

        let thread_pool = ThreadPool::new(thread_count);
        let reactor = Reactor::new();

        Self(Arc::new(Inner {
            thread_pool,
            reactor,
        }))
    }

    /// Зарегистрировать рантайм на текущем потоке
    pub fn register(self) {
        RUNTIME.with(|rt| {
            let borrow = rt.borrow();
            let mut rt = borrow.lock();

            if rt.is_some() {
                panic!("AsyncRuntime is already registered");
            };

            *rt = Some(self)
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
                .lock()
                .take()
                .expect("runtime is not registered");

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
                        // Пока главному потоку нечего делать, отдадим его на чтение событий реактором
                        rt.0.reactor.poll_events();
                    }
                }
            }

            rt.0.thread_pool.clone().join()
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
        self.schedule_task(Arc::clone(&task));

        JoinHandle::new(res_rx, task)
    }

    pub fn reactor(&self) -> &Reactor {
        &self.0.reactor
    }

    /// Запланировать задачу на выполнение
    fn schedule_task<T>(&self, task: Arc<Task<T>>)
    where
        T: Send + 'static,
    {
        self.0.thread_pool.spawn(move || Arc::clone(&task).poll());
    }
}
