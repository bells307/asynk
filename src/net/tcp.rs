use super::stream::TcpStream;
use crate::{reactor::event::RegisteredSource, reactor_global};
use futures::{Stream, StreamExt};
use mio::{net::TcpListener as MioTcpListener, Interest};
use std::{
    future::Future,
    io,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

type TcpListenerSource = RegisteredSource<MioTcpListener>;

pub struct TcpListener(TcpListenerSource);

impl TcpListener {
    pub fn bind(addr: SocketAddr) -> io::Result<Self> {
        let server = MioTcpListener::bind(addr)?;
        let source = reactor_global()
            .register(server, Interest::READABLE)
            .unwrap();
        Ok(Self(source))
    }

    pub fn accept(self) -> Accept {
        Accept(self.0)
    }
}

pub struct Accept(TcpListenerSource);

impl Future for Accept {
    type Output = io::Result<(TcpStream, SocketAddr)>;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        // Try to accept TCP connection
        match self.0.accept() {
            Ok((stream, addr)) => Poll::Ready(Ok((TcpStream::try_from(stream)?, addr))),
            Err(err) => {
                if matches!(err.kind(), io::ErrorKind::WouldBlock) {
                    // If accepting may block us, then we return Poll::Pending
                    Poll::Pending
                } else {
                    Poll::Ready(Err(err))
                }
            }
        }
    }
}

impl Stream for Accept {
    type Item = io::Result<(TcpStream, SocketAddr)>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.0.event_recv_mut().poll_next_unpin(cx) {
            Poll::Ready(event) => match event {
                Some(_) => self.poll(cx).map(Option::Some),
                None => Poll::Ready(None),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}
