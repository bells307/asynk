mod reactor;
mod rt;
mod tp;

pub use crate::rt::handle::JoinHandle;

use futures::Future;
use reactor::Reactor;
use rt::{builder::AsyncRuntimeBuilder, AsyncRuntime};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    OnceLock,
};

/// Global runtime
static RUNTIME: OnceLock<AsyncRuntime> = OnceLock::new();

/// Runtime terminated flag
static TERMINATED: AtomicBool = AtomicBool::new(false);

pub fn builder() -> AsyncRuntimeBuilder {
    AsyncRuntimeBuilder::new()
}

pub fn block_on<F>(fut: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    runtime().block_on(fut);
    TERMINATED.store(true, Ordering::SeqCst);
}

pub fn spawn<T, F>(fut: F) -> JoinHandle<T>
where
    T: Send + 'static,
    F: Future<Output = T> + Send + 'static,
{
    runtime().spawn_task(fut)
}

impl AsyncRuntime {
    /// Register runtime
    pub fn register(self) {
        check_terminated();

        if RUNTIME.set(self).is_err() {
            panic!("runtime register error");
        }
    }
}

pub(crate) fn reactor() -> &'static Reactor {
    runtime().reactor()
}

fn runtime() -> &'static AsyncRuntime {
    check_terminated();

    RUNTIME.get().expect("runtime is not set")
}

fn check_terminated() {
    if TERMINATED.load(Ordering::SeqCst) {
        panic!("runtime is already terminated");
    }
}
