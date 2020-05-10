// Integration tests

use tp::{Throughput, ThroughputSynchronized};

use std::sync::Arc;
use std::time::Instant;

#[test]
fn it_works() {
    let mut tp: Throughput<Instant> = Throughput::new();
    tp.report(1);
    tp.reset();

    let tp: Arc<ThroughputSynchronized<Instant>> = Arc::new(ThroughputSynchronized::new());
    tp.report(1);
    tp.reset();
}
