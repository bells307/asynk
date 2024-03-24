use super::Reactor;
use bitflags::bitflags;
use futures::{
    channel::mpsc::{self, TrySendError},
    SinkExt, Stream, StreamExt,
};
use mio::{event::Source, Token};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};

pub type EventSender = mpsc::UnboundedSender<Event>;
pub type EventReceiver = mpsc::UnboundedReceiver<Event>;

bitflags! {
    #[derive(Clone, Hash, Eq, PartialEq)]
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

impl Event {
    pub fn is_readable(&self) -> bool {
        *self == Event::READABLE
    }

    pub fn is_writable(&self) -> bool {
        *self == Event::WRITABLE
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

pub struct EventSource<S>
where
    S: Source,
{
    reactor: Reactor,
    token: Token,
    ev_tx: EventSender,
    ev_rx: EventReceiver,
    source: S,
    waiters: HashMap<Event, Vec<Waker>>,
}

impl<S> Unpin for EventSource<S> where S: Source {}

impl<S> EventSource<S>
where
    S: Source,
{
    pub fn new(reactor: Reactor, token: Token, source: S) -> Self {
        let (ev_tx, ev_rx) = mpsc::unbounded();

        Self {
            reactor,
            token,
            ev_tx,
            ev_rx,
            source,
            waiters: HashMap::new(),
        }
    }

    pub fn push_event(&self, ev: Event) -> Result<(), TrySendError<Event>> {
        self.ev_tx.unbounded_send(ev.clone())?;
        if let Some(waiters) = self.waiters.get(&ev) {
            for w in waiters {
                w.wake_by_ref();
            }
        }

        Ok(())
    }
}

impl<S> Stream for EventSource<S>
where
    S: Source,
{
    type Item = Event;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.ev_rx.poll_next_unpin(cx)
    }
}

impl<S> Deref for EventSource<S>
where
    S: Source,
{
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.source
    }
}

impl<S> DerefMut for EventSource<S>
where
    S: Source,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.source
    }
}

impl<S> Drop for EventSource<S>
where
    S: Source,
{
    fn drop(&mut self) {
        self.reactor.deregister(self.token, &mut self.source).ok();
    }
}
