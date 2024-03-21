use futures::{future, lock::Mutex};
use futures_timer::Delay;
use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

fn main() {
    asynk::builder().build().register();
    asynk::block_on(main_future());
}

async fn main_future() {
    let val = Arc::new(Mutex::new(AtomicU32::new(0)));
    let expected_val = 10_000;

    let handles = (0..expected_val)
        .map(|_| Arc::clone(&val))
        .map(|val| {
            asynk::spawn(async move {
                // some computations ...
                Delay::new(Duration::from_secs(1)).await;
                val.lock().await.fetch_add(1, Ordering::SeqCst);
            })
        })
        .collect::<Vec<_>>();

    future::join_all(handles).await;

    assert_eq!(val.lock().await.load(Ordering::SeqCst), expected_val);
}
