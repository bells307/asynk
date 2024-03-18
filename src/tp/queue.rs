use super::Job;
use parking_lot::{Condvar, Mutex};
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Default)]
pub struct JobQueue {
    queue: Mutex<VecDeque<Job>>,
    not_empty: Condvar,
    finished: AtomicBool,
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    /// Добавить задачу в очередь и уведомить ожидающий поток
    pub fn add(&self, job: Job) {
        if self.finished.load(Ordering::Acquire) {
            return;
        }

        self.queue.lock().push_back(job);
        self.not_empty.notify_one();
    }

    /// Установить флаг завершения и уведомить спящие потоки
    pub fn finish_ntf(&self) {
        self.finished.store(true, Ordering::Release);
        self.not_empty.notify_all();
    }

    /// Получение следующей задачи из очереди. Если очередь пуста - поток засыпает до
    /// появления новых элементов.
    ///
    /// Возвращает `None`, если очередь больше не выдаст элементов.
    pub fn get_blocked(&self) -> Option<Job> {
        if self.finished.load(Ordering::Acquire) {
            return None;
        }

        let mut lock = self.queue.lock();

        while lock.is_empty() {
            // Элементов в очереди нет, засыпаем
            self.not_empty.wait(&mut lock);
            // Возможно, нас разбудили потому что пора выходить
            if self.finished.load(Ordering::Acquire) {
                return None;
            }
        }

        // Сейчас мы можем предположить, что в очереди точно есть элементы
        assert!(!lock.is_empty());

        Some(lock.pop_front().expect("there must be prepared job(s)"))
    }
}
