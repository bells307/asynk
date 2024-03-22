pub mod event;

use self::event::{EventSender, RegisteredSource};
use futures::channel::mpsc;
use mio::{event::Source, Events, Interest, Poll, Token};
use parking_lot::RwLock;
use sharded_slab::Slab;
use std::{io, sync::Arc, time::Duration};

const POLL_EVENTS_TIMEOUT: Duration = Duration::from_millis(1);

#[derive(Clone)]
pub struct Reactor(Arc<Inner>);

struct Inner {
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

    /// Register interested events for the given source
    pub fn register<S>(&self, mut source: S, interests: Interest) -> io::Result<RegisteredSource<S>>
    where
        S: Source,
    {
        let (tx, rx) = mpsc::unbounded();

        let token = self
            .0
            .senders
            .insert(tx)
            .ok_or(io::Error::new(io::ErrorKind::Other, "slab queue is full"))?;

        let token = Token(token);

        self.0
            .poll
            .read()
            .registry()
            .register(&mut source, token, interests)?;

        Ok(RegisteredSource::new(self.clone(), token, rx, source))
    }

    pub fn poll_events(&self) -> io::Result<()> {
        let mut poll = self.0.poll.write();
        let mut events = self.0.events.write();

        poll.poll(&mut events, Some(POLL_EVENTS_TIMEOUT))?;

        for event in events.into_iter() {
            if let Some(tx) = self.0.senders.get(event.token().into()) {
                tx.unbounded_send(event.into()).unwrap();
            }
        }

        Ok(())
    }

    /// Remove the interests for the given source
    fn deregister<S>(&self, token: Token, source: &mut S) -> io::Result<()>
    where
        S: Source,
    {
        self.0.poll.read().registry().deregister(source)?;
        self.0.senders.get(token.0);
        Ok(())
    }
}
