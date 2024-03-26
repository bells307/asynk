use crate::{reactor::event::EventSource, reactor_global};
use futures::{AsyncRead, AsyncWrite, StreamExt};
use mio::{net::TcpStream as MioTcpStream, Interest};
use std::{
    io::{Error, ErrorKind, Read, Result, Write},
    pin::Pin,
    task::{Context, Poll},
};

pub struct TcpStream {
    source: EventSource<MioTcpStream>,
    maybe_readable: bool,
    maybe_writable: bool,
}

impl TcpStream {
    fn poll_events(mut self: Pin<&mut Self>, cx: &mut Context<'_>) {
        loop {
            match self.source.poll_next_unpin(cx) {
                Poll::Ready(Some(ev)) => {
                    if ev.is_readable() {
                        self.maybe_readable = true;
                    } else if ev.is_writable() {
                        self.maybe_writable = true;
                    }
                }
                Poll::Ready(None) => panic!("think about it"),
                Poll::Pending => break,
            }
        }
    }
}

impl TryFrom<MioTcpStream> for TcpStream {
    type Error = Error;

    fn try_from(stream: MioTcpStream) -> Result<Self> {
        let source = reactor_global().register(stream, Interest::READABLE | Interest::WRITABLE)?;
        Ok(Self {
            source,
            maybe_readable: false,
            maybe_writable: false,
        })
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        loop {
            match self.source.read(buf) {
                Ok(n) => {
                    break Poll::Ready(Ok(n));
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // The socket would block, so we can try to poll events for this socket
                    // and if it becomes readable, then try it again
                    self.maybe_readable = false;

                    self.as_mut().poll_events(cx);

                    if self.maybe_readable {
                        continue;
                    } else {
                        break Poll::Ready(Ok(0));
                    }
                }
                Err(err) => break Poll::Ready(Err(err)),
            }
        }
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        loop {
            match self.source.write(buf) {
                Ok(n) => {
                    break Poll::Ready(Ok(n));
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // The socket would block, so we can try to poll events for this socket
                    // and if it becomes readable, then try it again
                    self.maybe_writable = false;

                    self.as_mut().poll_events(cx);

                    if self.maybe_writable {
                        continue;
                    } else {
                        break Poll::Ready(Ok(0));
                    }
                }
                Err(err) => break Poll::Ready(Err(err)),
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        todo!()
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        todo!()
    }
}
