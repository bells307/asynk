mod event;

use self::event::{EventSender, EventSource};
use futures::channel::mpsc;
use mio::{event::Source, Events, Interest, Poll, Token};
use parking_lot::RwLock;
use sharded_slab::Slab;
use std::{sync::Arc, time::Duration};

#[derive(Clone)]
pub struct Reactor(Arc<Inner>);

struct Inner {
    /// Получатели event'ов
    senders: Slab<EventSender>,
    poll: RwLock<Poll>,
    events: RwLock<Events>,
}

impl Default for Reactor {
    fn default() -> Self {
        Self::new()
    }
}

impl Reactor {
    pub fn new() -> Self {
        Self(Arc::new(Inner {
            poll: RwLock::new(Poll::new().unwrap()),
            events: RwLock::new(Events::with_capacity(128)),
            senders: Slab::new(),
        }))
    }

    pub fn register<S>(&self, mut source: S, interests: Interest) -> EventSource<S>
    where
        S: Source,
    {
        let (tx, rx) = mpsc::unbounded();

        let token = self.0.senders.insert(tx).unwrap();
        let token = Token(token);

        self.0
            .poll
            .read()
            .registry()
            .register(&mut source, token, interests)
            .unwrap();

        EventSource::new(self.clone(), token, rx, source)
    }

    pub fn poll_events(&self) {
        let mut poll = self.0.poll.write();
        let mut events = self.0.events.write();

        poll.poll(&mut events, Some(Duration::from_micros(100)))
            .unwrap();

        for event in events.into_iter() {
            if let Some(tx) = self.0.senders.get(event.token().into()) {
                tx.unbounded_send(event.into()).unwrap();
            }
        }
    }

    fn deregister<S>(&self, token: Token, source: &mut S)
    where
        S: Source,
    {
        self.0.poll.read().registry().deregister(source).unwrap();
        self.0.senders.get(token.0);
    }
}
