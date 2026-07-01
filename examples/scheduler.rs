// MonadIO scheduler routing: `observe_on` / `subscribe_on` (RxJava-style).
//
// Demonstrates thread-hopping and the one real trap (verified in
// src/monadio.rs:141-163): with no `observe_on`, `subscribe_on` is a silent
// no-op and the effect runs synchronously on the caller thread.
//
// Run: cargo run --example scheduler --features=test_runtime

use std::sync::Arc;
use std::thread::{self, ThreadId};

use fp_rust::common::SubscriptionFunc;
use fp_rust::handler::{Handler, HandlerThread};
use fp_rust::monadio::MonadIO;
use fp_rust::sync::{BlockingQueue, Queue};

fn main() {
    let main_tid = thread::current().id();
    println!("main thread {:?}", main_tid);

    // Case 1 — routed: with `observe_on` set, the effect and `on_next` run on a
    // worker thread, not the caller. Capture the delivering thread id to prove it.
    let handler_observe_on = HandlerThread::new_with_mutex();
    let handler_subscribe_on = HandlerThread::new_with_mutex();
    {
        let mut h_ob = handler_observe_on.lock().unwrap();
        let mut h_sub = handler_subscribe_on.lock().unwrap();
        h_ob.start();
        h_sub.start();
    }

    let mut routed = MonadIO::new(|| 42);
    routed.observe_on(Some(handler_observe_on.clone()));
    routed.subscribe_on(Some(handler_subscribe_on.clone()));

    let delivered = BlockingQueue::<ThreadId>::new();
    let mut delivered_put = delivered.clone();
    routed.subscribe(Arc::new(SubscriptionFunc::new(move |x: Arc<u16>| {
        let tid = thread::current().id();
        println!("routed on_next {:?} on thread {:?}", x, tid);
        assert_eq!(42, *x);
        delivered_put.put(tid);
    })));

    // take() blocks until the worker delivers — deterministic, no sleep.
    let mut delivered_take = delivered.clone();
    let routed_tid = delivered_take.take().expect("routed delivery");
    assert_ne!(main_tid, routed_tid); // ran off the caller thread

    // Case 2 — the trap: `subscribe_on` WITHOUT `observe_on`. The subscribe
    // handler is ignored and the effect runs synchronously on the caller.
    let mut trap = MonadIO::new(|| 7);
    trap.subscribe_on(Some(handler_subscribe_on.clone())); // no observe_on!

    let trapped = BlockingQueue::<ThreadId>::new();
    let mut trapped_put = trapped.clone();
    trap.subscribe(Arc::new(SubscriptionFunc::new(move |x: Arc<u16>| {
        let tid = thread::current().id();
        println!("trap on_next {:?} on thread {:?}", x, tid);
        assert_eq!(7, *x);
        trapped_put.put(tid);
    })));

    // Synchronous: the value is already enqueued before subscribe() returned.
    let mut trapped_take = trapped.clone();
    let trap_tid = trapped_take.take().expect("trap delivery");
    assert_eq!(main_tid, trap_tid); // subscribe_on alone did nothing

    println!("scheduler: routed off-thread, trap ran on caller (as documented)");
}
