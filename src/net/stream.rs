use crate::{
    reactor::event::{Event, RegisteredSource},
    reactor_global,
};
use bitflags::bitflags;
use futures::{AsyncRead, AsyncWrite, StreamExt};
use mio::{net::TcpStream as MioTcpStream, Interest};
use std::{
    io::{Error, ErrorKind, Read, Result, Write},
    mem,
    pin::Pin,
    task::{Context, Poll},
};

pub struct TcpStream {
    source: RegisteredSource<MioTcpStream>,
    reading: bool,
    writing: bool,
}

impl TryFrom<MioTcpStream> for TcpStream {
    type Error = Error;

    fn try_from(stream: MioTcpStream) -> Result<Self> {
        let source = reactor_global().register(stream, Interest::READABLE | Interest::WRITABLE)?;
        Ok(Self {
            source,
            reading: false,
            writing: false,
        })
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        if mem::replace(&mut self.reading, true) {
            match self.source.read(buf) {
                Ok(n) => Poll::Ready(Ok(n)),
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    self.reading = false;
                    Poll::Ready(Ok(0))
                }
                Err(err) => Poll::Ready(Err(err)),
            }
        } else {
            loop {
                match self.source.event_recv_mut().poll_next_unpin(cx) {
                    Poll::Ready(Some(event)) => {
                        if event == Event::READABLE {
                            match self.source.read(buf) {
                                Ok(n) => return Poll::Ready(Ok(n)),
                                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                                    self.reading = false;
                                    return Poll::Ready(Ok(0));
                                }
                                Err(err) => return Poll::Ready(Err(err)),
                            }
                        } else {
                            // Continue try to get events from socket
                            continue;
                        }
                    }
                    // Events receiver is dropped
                    Poll::Ready(None) => panic!("?"),
                    // There are no more events
                    Poll::Pending => return Poll::Pending,
                }
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
        if mem::replace(&mut self.writing, true) {
            match self.source.write(buf) {
                Ok(n) => Poll::Ready(Ok(n)),
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    self.writing = false;
                    Poll::Ready(Ok(0))
                }
                Err(err) => Poll::Ready(Err(err)),
            }
        } else {
            loop {
                match self.source.event_recv_mut().poll_next_unpin(cx) {
                    Poll::Ready(Some(event)) => {
                        if event == Event::WRITABLE {
                            match self.source.write(buf) {
                                Ok(n) => return Poll::Ready(Ok(n)),
                                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                                    self.writing = false;
                                    return Poll::Ready(Ok(0));
                                }
                                Err(err) => return Poll::Ready(Err(err)),
                            }
                        } else {
                            // Continue try to get events from socket
                            continue;
                        }
                    }
                    // Events receiver is dropped
                    Poll::Ready(None) => panic!("?"),
                    // There are no more events
                    Poll::Pending => return Poll::Pending,
                }
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

bitflags! {
    struct TcpStreamState: u8 {
        const UNKNOWN = 1;
        const READY_READ = 1 << 1;
        const READY_WRITE = 1 << 2;
    }
}
