use async_runtime::Executor;
use futures_timer::Delay;
use std::time::Duration;

fn main() {
    Executor::default().register();

    Executor::block_on(|exec| async move {
        println!("start main");

        let jh = exec.spawn(|_| async move {
            println!("start spawned");
            Delay::new(Duration::from_secs(3)).await;
            println!("stop spawned");
            1
        });

        Delay::new(Duration::from_secs(1)).await;

        println!("stop main");

        let val = jh.await.unwrap();
        println!("spawned value: {val}");
    })
    .unwrap();
}
