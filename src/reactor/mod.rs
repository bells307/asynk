mod error;
mod event;

use self::{
    error::ReactorError,
    event::{EventSender, EventSource},
};
use futures::channel::mpsc;
use mio::{event::Source, Events, Interest, Poll, Token};
use parking_lot::RwLock;
use sharded_slab::Slab;
use std::{sync::Arc, time::Duration};

const POLL_EVENTS_TIMEOUT: Duration = Duration::from_micros(100);

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
    pub fn register<S>(
        &self,
        mut source: S,
        interests: Interest,
    ) -> Result<EventSource<S>, ReactorError>
    where
        S: Source,
    {
        let (tx, rx) = mpsc::unbounded();

        let token = self
            .0
            .senders
            .insert(tx)
            .ok_or(ReactorError::SlabQueueFull)?;

        let token = Token(token);

        self.0
            .poll
            .read()
            .registry()
            .register(&mut source, token, interests)?;

        Ok(EventSource::new(self.clone(), token, rx, source))
    }

    pub fn poll_events(&self) -> Result<(), ReactorError> {
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
    fn deregister<S>(&self, token: Token, source: &mut S) -> Result<(), ReactorError>
    where
        S: Source,
    {
        self.0.poll.read().registry().deregister(source)?;
        self.0.senders.get(token.0);
        Ok(())
    }
}
