use super::stream::TcpStream;
use crate::reactor_global;
use futures::Stream;
use mio::{net::TcpListener as MioTcpListener, Interest, Token};
use std::{
    io,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

pub struct TcpListener(MioTcpListener);

impl TcpListener {
    pub fn bind(addr: SocketAddr) -> io::Result<Self> {
        Ok(Self(MioTcpListener::bind(addr)?))
    }

    pub fn accept(self) -> Accept {
        Accept::new(self.0)
    }
}

pub struct Accept {
    source: MioTcpListener,
    token: Option<Token>,
}

impl Accept {
    pub fn new(source: MioTcpListener) -> Self {
        Self {
            source,
            token: None,
        }
    }
}

impl Stream for Accept {
    type Item = io::Result<(TcpStream, SocketAddr)>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.token.is_none() {
            let token = reactor_global().register(
                &mut self.source,
                Interest::READABLE,
                cx.waker().clone(),
            )?;

            self.token = Some(token);
            return Poll::Pending;
        }

        match self.source.accept() {
            Ok((stream, addr)) => Poll::Ready(Some(Ok((stream.try_into()?, addr)))),
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
            Err(e) => Poll::Ready(Some(Err(e))),
        }
    }
}

impl Drop for Accept {
    fn drop(&mut self) {
        if let Some(token) = self.token {
            reactor_global()
                .deregister(token, &mut self.source)
                .unwrap();
        }
    }
}
