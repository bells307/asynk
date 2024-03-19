use futures::channel::mpsc;
use mio::{Events, Poll};
use sharded_slab::Slab;
use std::thread;

pub struct Reactor {
    /// Получатели event'ов
    receivers: Slab<mpsc::UnboundedReceiver<Event>>,
}

impl Reactor {
    pub fn new() -> Self {
        let thread = thread::spawn(Self::recv_events_thread_routine);
        Self {
            receivers: Slab::new(),
        }
    }

    fn recv_events_thread_routine() {
        let mut poll = Poll::new().unwrap();
        let mut events = Events::with_capacity(128);
        loop {}
    }
}

struct Event;
