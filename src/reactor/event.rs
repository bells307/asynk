use super::Reactor;
use bitflags::bitflags;
use futures::channel::mpsc;
use mio::{event::Source, Token};
use std::ops::{Deref, DerefMut};

pub type EventSender = mpsc::UnboundedSender<Event>;
pub type EventReceiver = mpsc::UnboundedReceiver<Event>;

bitflags! {
    #[derive(Eq, PartialEq)]
    pub struct Event: u8 {
        const READABLE = 1;
        const WRITABLE = 1 << 1;
        const ERROR = 1 << 2;
        const READ_CLOSED = 1 << 3;
        const WRITE_CLOSED = 1 << 4;
        const PRIORITY = 1 << 5;
        const AIO = 1 << 6;
        const LIO = 1 << 7;
    }
}

impl From<&mio::event::Event> for Event {
    fn from(ev: &mio::event::Event) -> Self {
        let mut flags = 0;

        flags |= u8::from(ev.is_readable());
        flags |= u8::from(ev.is_writable()) << 1;
        flags |= u8::from(ev.is_error()) << 2;
        flags |= u8::from(ev.is_read_closed()) << 3;
        flags |= u8::from(ev.is_write_closed()) << 4;
        flags |= u8::from(ev.is_priority()) << 5;
        flags |= u8::from(ev.is_aio()) << 6;
        flags |= u8::from(ev.is_lio()) << 7;

        Event::from_bits_retain(flags)
    }
}

pub struct RegisteredSource<S>
where
    S: Source,
{
    reactor: Reactor,
    token: Token,
    event_recv: EventReceiver,
    source: S,
}

impl<S> RegisteredSource<S>
where
    S: Source,
{
    pub fn new(reactor: Reactor, token: Token, event_recv: EventReceiver, source: S) -> Self {
        Self {
            reactor,
            token,
            event_recv,
            source,
        }
    }

    pub fn event_recv_mut(&mut self) -> &mut EventReceiver {
        &mut self.event_recv
    }
}

impl<S> Deref for RegisteredSource<S>
where
    S: Source,
{
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.source
    }
}

impl<S> DerefMut for RegisteredSource<S>
where
    S: Source,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.source
    }
}

impl<S> Drop for RegisteredSource<S>
where
    S: Source,
{
    fn drop(&mut self) {
        self.reactor.deregister(self.token, &mut self.source).ok();
    }
}
