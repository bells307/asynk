use asynk::net::tcp::TcpListener;
use futures::{AsyncReadExt, StreamExt};

fn main() {
    asynk::builder().build().register();
    asynk::block_on(main_future());
}

async fn main_future() {
    let addr = "127.0.0.1:8040".parse().unwrap();

    let listener = TcpListener::bind(addr).unwrap();
    let mut stream = listener.accept();

    let (mut stream, addr) = stream.next().await.unwrap().unwrap();

    println!("got connection from addr: {}", addr);

    let mut buf = vec![];
    stream.read_to_end(&mut buf).await.unwrap();

    println!("{}", String::from_utf8_lossy(&buf));
}
