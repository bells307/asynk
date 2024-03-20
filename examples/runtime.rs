use asynk::AsyncRuntime;
use futures_timer::Delay;
use std::{thread, time::Duration};

// fn main() {
//     AsyncRuntime::builder().build();

//     AsyncRuntime::block_on(|rt| async move {
//         let thr_id = || thread::current().id();

//         println!("start main task, thread: {:?}", thr_id());

//         let jh = rt.spawn(|_| async move {
//             println!("start spawned task, thread: {:?}", thr_id());
//             Delay::new(Duration::from_secs(3)).await;
//             println!("stop spawned task, thread: {:?}", thr_id());
//             1
//         });

//         Delay::new(Duration::from_secs(1)).await;

//         println!("stop main task, thread: {:?}", thr_id());

//         let val = jh.await.unwrap();
//         println!("spawned task returned: {}, thread: {:?}", val, thr_id());
//     })
//     .unwrap();
// }

fn main() {
    AsyncRuntime::builder().build();

    AsyncRuntime::block_on(|rt| async move {
        let thr_id = || thread::current().id();

        println!("start main task, thread: {:?}", thr_id());

        let jh = rt.spawn(|rt| async move {
            let jh = rt.spawn(|_| async {
                println!("one more inner task start!");
                Delay::new(Duration::from_secs(3)).await;
                println!("one more inner task end!");
                1
            });

            println!("start spawned task, thread: {:?}", thr_id());
            Delay::new(Duration::from_secs(3)).await;
            println!("stop spawned task, thread: {:?}", thr_id());
            jh.await
        });

        Delay::new(Duration::from_secs(1)).await;

        println!("stop main task, thread: {:?}", thr_id());

        let res = jh.await.unwrap();
        println!("spawned task returned: {:?}, thread: {:?}", res, thr_id());
    })
    .unwrap();
}
