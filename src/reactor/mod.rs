pub mod io_handle;

use mio::{event::Source, Events, Interest, Poll, Token};
use parking_lot::RwLock;
use sharded_slab::Slab;
use std::{
    io::{self, Error},
    sync::Arc,
    task::Waker,
    time::Duration,
};

const POLL_EVENTS_TIMEOUT: Duration = Duration::from_millis(1);

#[derive(Clone)]
pub struct Reactor(Arc<Inner>);

struct Inner {
    wakers: Slab<Waker>,
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
            wakers: Slab::new(),
        }))
    }

    /// Register interested events for the given source
    pub fn register<S>(
        &self,
        source: &mut S,
        interests: Interest,
        waker: Waker,
    ) -> io::Result<Token>
    where
        S: Source,
    {
        let token = self
            .0
            .wakers
            .insert(waker)
            .ok_or(Error::other("slab queue is full"))?;

        let token = Token(token);

        self.0
            .poll
            .read()
            .registry()
            .register(source, token, interests)?;

        Ok(token)
    }

    pub fn reregister<S>(
        &self,
        token: Token,
        source: &mut S,
        interests: Interest,
        waker: Waker,
    ) -> io::Result<Token>
    where
        S: Source,
    {
        self.0.wakers.remove(token.into());

        let new_token = Token(
            self.0
                .wakers
                .insert(waker)
                .ok_or(Error::other("slab queue is full"))?,
        );

        self.0
            .poll
            .read()
            .registry()
            .reregister(source, new_token, interests)?;

        Ok(new_token)
    }

    pub fn poll_events(&self) -> io::Result<()> {
        let mut poll = self.0.poll.write();
        let mut events = self.0.events.write();

        poll.poll(&mut events, Some(POLL_EVENTS_TIMEOUT))?;

        for event in events.into_iter() {
            if let Some(waker) = self.0.wakers.get(event.token().into()) {
                waker.wake_by_ref();
            }
        }

        Ok(())
    }

    /// Remove the interests for the given source
    pub fn deregister<S>(&self, token: Token, source: &mut S) -> io::Result<()>
    where
        S: Source,
    {
        self.0.poll.read().registry().deregister(source)?;
        self.0.wakers.get(token.0);
        Ok(())
    }
}
