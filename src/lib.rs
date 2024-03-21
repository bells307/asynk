mod reactor;
mod rt;
mod tp;

pub use crate::rt::handle::JoinHandle;

use futures::Future;
use rt::{builder::AsyncRuntimeBuilder, AsyncRuntime};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    OnceLock,
};

static TERMINATED: AtomicBool = AtomicBool::new(false);
static RUNTIME: OnceLock<AsyncRuntime> = OnceLock::new();

pub fn builder() -> AsyncRuntimeBuilder {
    AsyncRuntimeBuilder::new()
}

pub fn runtime() -> &'static AsyncRuntime {
    if TERMINATED.load(Ordering::SeqCst) {
        panic!("runtime is already terminated");
    }

    RUNTIME.get().expect("runtime is not set")
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
        if RUNTIME.set(self).is_err() {
            panic!("runtime register error");
        }
    }
}
