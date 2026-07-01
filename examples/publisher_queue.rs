// Publisher push -> pull bridge: `as_blocking_queue` / `subscribe_blocking_queue`.
//
// Turns a push-based pub/sub stream into a pull-based `BlockingQueue` so a
// consumer drains values on its own schedule instead of running in a callback.
// Note: this is a delivery bridge, not bounded backpressure (unbounded buffer).
//
// Run: cargo run --example publisher_queue --features=test_runtime

use std::sync::Arc;
use std::thread;

use fp_rust::publisher::Publisher;
use fp_rust::sync::Queue;

fn main() {
    let mut publisher = Publisher::new();

    // Bridge the stream into a pull-based queue. `subscription` keeps delivery
    // alive; dropping it would unsubscribe.
    let (_subscription, queue) = publisher.as_blocking_queue();

    // Consumer pulls on its own thread; take() blocks until an item arrives.
    let mut consumer_queue = queue.clone();
    let consumer = thread::spawn(move || {
        let mut received = Vec::new();
        for _ in 0..3 {
            let v: Arc<i32> = consumer_queue.take().expect("value");
            println!("consumer pulled {:?}", v);
            received.push(*v);
        }
        received
    });

    // Producer publishes at its own pace; values buffer in the queue until
    // pulled. No handler => delivery is synchronous on this publish() call.
    for i in 1..=3 {
        publisher.publish(i);
    }

    let received = consumer.join().expect("consumer thread");
    assert_eq!(vec![1, 2, 3], received);
    println!("publisher_queue: pulled {:?} in order", received);
}
