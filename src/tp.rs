use std::option::Option;
use std::sync::Mutex;
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
        Self {
            sum: 0,
            initial_time: T::now(),
        }
    }

    pub fn report(&mut self, value: u32) {
        self.sum += value;
    }

    pub fn reset(&mut self) {
        self.initial_time = T::now();
        self.sum = 0;
    }

    pub fn throughput(&mut self) -> Option<f64> {
        let elapsed: Duration = self.initial_time.elapsed();
        let denominator =
            f64::from(elapsed.as_secs() as u32) + f64::from(elapsed.subsec_millis()) / 1000.0;
        let tp = if denominator == 0.0 {
            None
        } else {
            Some(f64::from(self.sum) / denominator)
        };

        self.reset();

        tp
    }
}

impl<T: TimeSource> Default for Throughput<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ThroughputSynchronized<T: TimeSource> {
    tp_unsynchronized: Mutex<Throughput<T>>,
}

impl<T: TimeSource> ThroughputSynchronized<T> {
    pub fn new() -> Self {
        Self {
            tp_unsynchronized: Mutex::new(Throughput::new()),
        }
    }

    pub fn report(&self, value: u32) {
        self.tp_unsynchronized.lock().unwrap().report(value);
    }

    pub fn reset(&self) {
        self.tp_unsynchronized.lock().unwrap().reset();
    }

    pub fn throughput(&self) -> Option<f64> {
        self.tp_unsynchronized.lock().unwrap().throughput()
    }
}

impl<T: TimeSource> Default for ThroughputSynchronized<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "async")]
pub mod tokio_async {
    use tokio::sync::Mutex;

    pub struct ThroughputAsyncSynchronized<T: super::TimeSource> {
        tp_unsynchronized: Mutex<super::Throughput<T>>,
    }

    impl<T: super::TimeSource> ThroughputAsyncSynchronized<T> {
        pub fn new() -> Self {
            Self {
                tp_unsynchronized: Mutex::new(super::Throughput::new()),
            }
        }

        pub async fn report(&self, value: u32) {
            self.tp_unsynchronized.lock().await.report(value);
        }

        pub async fn reset(&self) {
            self.tp_unsynchronized.lock().await.reset();
        }

        pub async fn throughput(&self) -> Option<f64> {
            self.tp_unsynchronized.lock().await.throughput()
        }
    }

    impl<T: super::TimeSource> Default for ThroughputAsyncSynchronized<T> {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_approx_eq::assert_approx_eq;

    use std::option::Option;
    use std::sync::{Arc, Barrier, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    use tokio::runtime::Runtime;
    use tokio::time::{sleep, sleep_until};

    struct FakeInstant {}

    impl super::TimeSource for FakeInstant {
        fn now() -> Self {
            FakeInstant {}
        }

        fn elapsed(&self) -> Duration {
            Duration::new(10, 0)
        }
    }

    struct ZeroTimeFakeInstant {}

    impl super::TimeSource for ZeroTimeFakeInstant {
        fn now() -> Self {
            ZeroTimeFakeInstant {}
        }

        fn elapsed(&self) -> Duration {
            Duration::new(0, 0)
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

        assert_approx_eq!(tp.throughput().unwrap(), 0.2);
    }

    #[test]
    fn test_zero_time() {
        let mut tp: super::Throughput<ZeroTimeFakeInstant> = super::Throughput::new();

        assert_eq!(None, tp.throughput());

        tp.report(1);

        assert_eq!(Duration::default(), Duration::new(0, 0));
        assert_eq!(None, tp.throughput());
    }

    #[test]
    fn test_in_threads() {
        let tp: Arc<Mutex<super::Throughput<Instant>>> =
            Arc::new(Mutex::new(super::Throughput::new()));

        let t1 = {
            let tp1 = tp.clone();
            thread::spawn(move || {
                tp1.lock().unwrap().report(1);
            })
        };

        let t2 = {
            let tp2 = tp.clone();
            thread::spawn(move || {
                tp2.lock().unwrap().throughput();
            })
        };

        let _ = t1.join();
        let _ = t2.join();
    }

    #[test]
    fn test_tp_synchronized_in_threads() {
        let tp: Arc<super::ThroughputSynchronized<FakeInstant>> =
            Arc::new(super::ThroughputSynchronized::new());
        let barrier = Arc::new(Barrier::new(2));

        let t1 = {
            let tp = tp.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                tp.report(1);
                barrier.wait();
            })
        };

        let t2 = {
            let tp = tp.clone();
            let barrier = barrier.clone();
            thread::spawn(move || -> Option<f64> {
                barrier.wait();
                tp.throughput()
            })
        };

        let _ = t1.join();
        assert_approx_eq!(t2.join().unwrap().unwrap(), 0.1);
    }

    #[test]
    fn test_delay() {
        let rt = Runtime::new().unwrap();

        rt.block_on(async {
            sleep(tokio::time::Duration::from_millis(10)).await;
            sleep_until(tokio::time::Instant::now() + tokio::time::Duration::from_millis(10)).await;
        });
    }

    #[tokio::test]
    async fn test_async_delay() {
        sleep(tokio::time::Duration::from_millis(10)).await;
        sleep_until(tokio::time::Instant::now() + tokio::time::Duration::from_millis(10)).await;
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_async_tp() {
        let tp: super::tokio_async::ThroughputAsyncSynchronized<FakeInstant> =
            super::tokio_async::ThroughputAsyncSynchronized::new();
        tp.report(1).await;
        tp.report(1).await;

        assert_approx_eq!(tp.throughput().await.unwrap(), 0.2);
    }
}
