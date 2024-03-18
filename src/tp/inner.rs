use super::{queue::JobQueue, JoinError};
use drop_panic::DropPanic;
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    sync::Arc,
    thread::{self, JoinHandle, ThreadId},
};

pub(super) struct Inner {
    job_queue: JobQueue,
    threads: Mutex<Option<HashMap<ThreadId, JoinHandle<()>>>>,
}

impl Inner {
    pub fn new(thread_count: usize) -> Arc<Self> {
        let this = Arc::new(Self {
            job_queue: JobQueue::new(),
            threads: Mutex::new(None),
        });

        for _ in 0..thread_count {
            Arc::clone(&this).create_worker();
        }

        this
    }

    /// Поставить задачу в очередь на исполнение
    pub fn spawn(&self, job: impl Fn() + Send + 'static) {
        self.job_queue.add(Box::new(job));
    }

    pub fn join(&self) -> Result<(), JoinError> {
        // Сообщаем очереди о завершении и просим ее разбудить потоки, которые
        // ожидают появления задач
        self.job_queue.finish_ntf();

        let mut panicked = 0;

        let Some(threads) = self.threads.lock().take() else {
            return Ok(());
        };

        threads.into_values().for_each(|jh| {
            jh.join().inspect_err(|_| panicked += 1).ok();
        });

        if panicked == 0 {
            Ok(())
        } else {
            Err(JoinError(panicked))
        }
    }

    /// Создать поток для выполнения задач
    fn create_worker(self: Arc<Self>) {
        let this = Arc::clone(&self);

        let jh = thread::spawn(move || {
            // Создаем объект, который в случае паники потока вызовет функцию восстановления
            let _dp = DropPanic::new(|| Arc::clone(&this).panic_handler());
            // Запуск рабочего цикла
            Arc::clone(&this).worker_routine()
        });

        let id = jh.thread().id();

        let mut lock = self.threads.lock();
        match *lock {
            Some(ref mut threads) => match threads.get_mut(&id) {
                Some(_) => panic!("thread with id {:?} already created", id),
                None => {
                    threads.insert(id, jh);
                }
            },
            None => *lock = Some(HashMap::from([(id, jh)])),
        }
    }

    /// Обработка паники рабочего потока
    fn panic_handler(self: Arc<Self>) {
        let id = thread::current().id();

        if let Some(threads) = self.threads.lock().as_mut() {
            threads.remove(&id);
        };

        // Пересоздаем рабочий поток
        self.create_worker();
    }

    /// Рабочий цикл воркера
    fn worker_routine(&self) {
        loop {
            let Some(job) = self.job_queue.get_blocked() else {
                return;
            };

            job();
        }
    }
}
