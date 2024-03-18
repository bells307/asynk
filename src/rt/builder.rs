use crate::rt::AsyncRuntime;

#[derive(Default)]
pub struct AsyncRuntimeBuilder {
    thread_count: Option<usize>,
}

impl AsyncRuntimeBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn thread_count(mut self, val: usize) -> Self {
        let val = if val == 0 {
            Self::default_thread_count()
        } else {
            val
        };

        self.thread_count = Some(val);
        self
    }

    pub fn build(self) {
        let thread_count = self.thread_count.unwrap_or_else(Self::default_thread_count);
        AsyncRuntime::new(thread_count).register();
    }

    fn default_thread_count() -> usize {
        num_cpus::get()
    }
}
