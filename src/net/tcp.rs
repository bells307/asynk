use super::stream::TcpStream;
use crate::{
    reactor::event::{Event, EventSource},
    reactor_global,
};
use futures::{Stream, StreamExt};
use mio::{net::TcpListener as MioTcpListener, Interest};
use std::{
    io,
    net::SocketAddr,
    pin::Pin,
    task::{ready, Context, Poll},
};

pub struct TcpListener(EventSource<MioTcpListener>);

impl TcpListener {
    pub fn bind(addr: SocketAddr) -> io::Result<Self> {
        let listener = MioTcpListener::bind(addr)?;
        let source = reactor_global()
            .register(listener, Interest::READABLE)
            .unwrap();

        Ok(Self(source))
    }

    pub fn accept(self) -> Accept {
        Accept(self.0)
    }
}

pub struct Accept(EventSource<MioTcpListener>);

impl Stream for Accept {
    type Item = io::Result<(TcpStream, SocketAddr)>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match ready!(self.0.poll_next_unpin(cx)) {
            Some(ev) if ev == Event::READABLE => match self.0.accept() {
                Ok((stream, addr)) => Poll::Ready(Some(Ok((TcpStream::try_from(stream)?, addr)))),
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
                Err(e) => Poll::Ready(Some(Err(e))),
            },
            Some(_) => Poll::Pending,
            None => Poll::Ready(None),
        }
    }
}
