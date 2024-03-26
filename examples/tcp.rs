use asynk::net::tcp::TcpListener;
use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};

const RESPONSE: &str = "HTTP/1.1 200 OK
Content-Type: text/html
Connection: keep-alive
Content-Length: 23

<h1>Hello, world!</h1>
";

fn main() {
    asynk::builder().build().register();
    asynk::block_on(main_future());
}

async fn main_future() {
    let addr = "127.0.0.1:8040".parse().unwrap();

    let listener = TcpListener::bind(addr).unwrap();
    let mut accept = listener.accept();

    while let Some(res) = accept.next().await {
        asynk::spawn(async move {
            let (mut stream, addr) = res.unwrap();

            println!("got connection from addr: {}", addr);

            let mut buf = vec![];

            stream.read_to_end(&mut buf).await.unwrap();

            println!("{}", String::from_utf8_lossy(&buf));

            stream.write_all(RESPONSE.as_bytes()).await.unwrap();

            stream.flush().await.unwrap();

            println!("drop connection");
        });
    }
}
