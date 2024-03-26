use crate::reactor_global;
use futures::{AsyncRead, AsyncWrite};
use mio::{net::TcpStream as MioTcpStream, Interest, Token};
use std::{
    io::{Error, ErrorKind, Read, Result, Write},
    pin::Pin,
    task::{Context, Poll},
};

pub struct TcpStream {
    source: MioTcpStream,
    read: bool,
    write: bool,
    token: Option<Token>,
}

impl TryFrom<MioTcpStream> for TcpStream {
    type Error = Error;

    fn try_from(stream: MioTcpStream) -> Result<Self> {
        Ok(Self {
            source: stream,
            read: false,
            write: false,
            token: None,
        })
    }
}

impl AsyncRead for TcpStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        if !self.read {
            match self.token {
                Some(token) => {
                    reactor_global().reregister(
                        token,
                        &mut self.source,
                        Interest::READABLE,
                        cx.waker().clone(),
                    )?;
                }
                None => {
                    let token = reactor_global().register(
                        &mut self.source,
                        Interest::READABLE,
                        cx.waker().clone(),
                    )?;

                    self.token = Some(token);
                }
            }

            self.read = true;
            return Poll::Pending;
        }

        match self.source.read(buf) {
            Ok(n) => {
                if n == 0 {
                    self.read = false;
                }

                Poll::Ready(Ok(n))
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                self.read = false;
                Poll::Ready(Ok(0))
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

impl AsyncWrite for TcpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        if !self.write {
            match self.token {
                Some(token) => {
                    reactor_global().reregister(
                        token,
                        &mut self.source,
                        Interest::WRITABLE,
                        cx.waker().clone(),
                    )?;
                }
                None => {
                    let token = reactor_global().register(
                        &mut self.source,
                        Interest::WRITABLE,
                        cx.waker().clone(),
                    )?;

                    self.token = Some(token);
                }
            }

            self.write = true;
            return Poll::Pending;
        }

        match self.source.write(buf) {
            Ok(n) => {
                if n == 0 {
                    self.write = false;
                }

                Poll::Ready(Ok(n))
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                self.write = false;
                Poll::Ready(Ok(0))
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        todo!()
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        todo!()
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        if let Some(token) = self.token {
            reactor_global()
                .deregister(token, &mut self.source)
                .unwrap();
        }
    }
}
