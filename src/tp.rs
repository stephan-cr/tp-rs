use std::time::{Duration, Instant};

pub trait TimeSource {
    fn now() -> Self;
    fn elapsed(&self) -> Duration;
}

impl TimeSource for Instant {
    fn now() -> Self {
        Instant::now()
    }

    fn elapsed(&self) -> Duration {
        Instant::elapsed(self)
    }
}

pub struct Throughput<T: TimeSource> {
    initial_time: T,
    sum: u32,
}

impl<T: TimeSource> Throughput<T> {
    pub fn new() -> Self {
        Throughput {
            sum: 0,
            initial_time: T::now(),
        }
    }

    pub fn report(&mut self, value: u32) {
        self.sum += value;
    }

    pub fn throughput(&mut self) -> f64 {
        let elapsed: Duration = self.initial_time.elapsed();
        let tp = f64::from(self.sum) / f64::from(elapsed.as_secs() as u32)
            + (f64::from(elapsed.subsec_millis()) / 1000.0);
        self.initial_time = T::now();
        self.sum = 0;

        tp
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    struct FakeInstant {}

    impl super::TimeSource for FakeInstant {
        fn now() -> Self {
            FakeInstant {}
        }

        fn elapsed(&self) -> Duration {
            Duration::new(10, 0)
        }
    }

    #[test]
    fn test_basic() {
        let mut tp: super::Throughput<Instant> = super::Throughput::new();
        tp.report(1);
        tp.report(1);

        tp.throughput();

        let mut tp: super::Throughput<FakeInstant> = super::Throughput::new();
        tp.report(1);
        tp.report(1);

        tp.throughput();
    }

    #[test]
    fn test_in_threads() {
        let tp: Arc<Mutex<super::Throughput<Instant>>> =
            Arc::new(Mutex::new(super::Throughput::new()));
        let tp1 = tp.clone();
        let t1 = thread::spawn(move || -> () {
            tp1.lock().unwrap().report(1);
        });

        let tp2 = tp.clone();
        let t2 = thread::spawn(move || -> () {
            tp2.lock().unwrap().throughput();
        });

        let _ = t1.join();
        let _ = t2.join();
    }
}
