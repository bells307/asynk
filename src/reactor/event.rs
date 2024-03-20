use super::Reactor;
use futures::channel::mpsc;
use mio::{event::Source, Token};

pub type EventSender = mpsc::UnboundedSender<Event>;
pub type EventReceiver = mpsc::UnboundedReceiver<Event>;

pub struct Event {}

impl From<&mio::event::Event> for Event {
    fn from(value: &mio::event::Event) -> Self {
        todo!()
    }
}

pub struct EventSource<S>
where
    S: Source,
{
    reactor: Reactor,
    token: Token,
    events: EventReceiver,
    source: S,
}

impl<S> EventSource<S>
where
    S: Source,
{
    pub fn new(reactor: Reactor, token: Token, events: EventReceiver, source: S) -> Self {
        Self {
            reactor,
            token,
            events,
            source,
        }
    }

    pub fn events_mut(&mut self) -> &mut EventReceiver {
        &mut self.events
    }
}

impl<S> Drop for EventSource<S>
where
    S: Source,
{
    fn drop(&mut self) {
        self.reactor.deregister(self.token, &mut self.source);
    }
}
