/*!
In this module there're implementations & tests
of general async handling features.
*/

use std::error::Error;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
    mpsc::RecvTimeoutError,
    Arc, Condvar, Mutex,
};
use std::time::Duration;

#[cfg(feature = "for_futures")]
use super::common::shared_thread_pool;
#[cfg(feature = "for_futures")]
use crate::futures::task::SpawnExt;
#[cfg(feature = "for_futures")]
use std::future::Future;
#[cfg(feature = "for_futures")]
use std::pin::Pin;
#[cfg(feature = "for_futures")]
use std::task::{Context, Poll, Waker};

use super::common::{Observable, RawFunc, SubscriptionFunc};

#[cfg(feature = "handler")]
use super::handler::{Handler, HandlerThread};
#[cfg(feature = "publisher")]
use super::publisher::Publisher;

/**
`Will` `trait` defines the interface which could do actions in its `Handler`.

# Remarks

This is highly inspired by `Java Future` concepts.

*/
pub trait Will<T>: Send + Sync + 'static {
    /**
    Did this `Will` start?
    Return `true` when it did started (no matter it has stopped or not)

    */
    fn is_started(&mut self) -> bool;

    /**
    Is this `Will` alive?
    Return `true` when it has started and not stopped yet.
    */
    fn is_alive(&mut self) -> bool;

    /**
    Start `Will`.
    */
    fn start(&mut self);

    /**
    Stop `Will`.
    */
    fn stop(&mut self);

    /**
    Add a callback called when it has completed.

    # Arguments

    * `subscription` - The callback.
    ``
    */
    fn add_callback(&mut self, subscription: Arc<SubscriptionFunc<T>>);

    /**
    Remove a callback called when it has completed.

    # Arguments

    * `subscription` - The callback.
    ``
    */
    fn remove_callback(&mut self, subscription: Arc<SubscriptionFunc<T>>);

    /**
    Get the result.
    */
    fn result(&mut self) -> Option<T>;
}

#[cfg(all(feature = "publisher", feature = "handler"))]
#[derive(Clone)]
pub struct WillAsync<T> {
    effect: Arc<Mutex<dyn FnMut() -> T + Send + Sync + 'static>>,
    handler: Arc<Mutex<dyn Handler>>,
    publisher: Arc<Mutex<Publisher<T>>>,
    started_alive: Arc<Mutex<(AtomicBool, AtomicBool)>>,
    result: Arc<Mutex<Option<T>>>,

    #[cfg(feature = "for_futures")]
    waker: Arc<Mutex<Option<Waker>>>,
}

#[cfg(all(feature = "publisher", feature = "handler"))]
impl<T> WillAsync<T> {
    pub fn new(effect: impl FnMut() -> T + Send + Sync + 'static) -> WillAsync<T> {
        Self::new_with_handler(effect, HandlerThread::new_with_mutex())
    }
    pub fn new_with_handler(
        effect: impl FnMut() -> T + Send + Sync + 'static,
        handler: Arc<Mutex<dyn Handler>>,
    ) -> WillAsync<T> {
        WillAsync {
            handler,
            effect: Arc::new(Mutex::new(effect)),
            started_alive: Arc::new(Mutex::new((AtomicBool::new(false), AtomicBool::new(false)))),
            publisher: Arc::new(Mutex::new(Publisher::default())),
            result: Arc::new(Mutex::new(None)),

            #[cfg(feature = "for_futures")]
            waker: Arc::new(Mutex::new(None)),
        }
    }
}

#[cfg(all(feature = "publisher", feature = "handler"))]
impl<T> Will<T> for WillAsync<T>
where
    T: Clone + Send + Sync + 'static,
{
    fn is_started(&mut self) -> bool {
        let started_alive = self.started_alive.lock().unwrap();
        let &(ref started, _) = &*started_alive;
        started.load(Ordering::SeqCst)
    }

    fn is_alive(&mut self) -> bool {
        let started_alive = self.started_alive.lock().unwrap();
        let &(_, ref alive) = &*started_alive;
        alive.load(Ordering::SeqCst)
    }

    fn start(&mut self) {
        let started_alive = self.started_alive.lock().unwrap();
        let &(ref started, ref alive) = &*started_alive;
        if started.load(Ordering::SeqCst) {
            return;
        }
        started.store(true, Ordering::SeqCst);
        if alive.load(Ordering::SeqCst) {
            return;
        }
        alive.store(true, Ordering::SeqCst);

        let mut this = self.clone();
        let _effect = self.effect.clone();
        let _publisher = self.publisher.clone();

        let mut handler = self.handler.lock().unwrap();
        handler.start();
        handler.post(RawFunc::new(move || {
            let result = { (_effect.lock().unwrap())() };
            {
                (*this.result.lock().unwrap()) = Some(result.clone());
            }
            {
                _publisher.lock().unwrap().publish(result);
            }
            this.stop();
        }));
    }

    fn stop(&mut self) {
        let started_alive = self.started_alive.lock().unwrap();
        let &(ref started, ref alive) = &*started_alive;
        if !started.load(Ordering::SeqCst) {
            return;
        }
        if !alive.load(Ordering::SeqCst) {
            return;
        }
        alive.store(false, Ordering::SeqCst);

        #[cfg(feature = "for_futures")]
        {
            if let Some(waker) = self.waker.lock().unwrap().take() {
                waker.wake()
            }
        }
    }

    fn add_callback(&mut self, subscription: Arc<SubscriptionFunc<T>>) {
        self.publisher.lock().unwrap().subscribe(subscription);
    }

    fn remove_callback(&mut self, subscription: Arc<SubscriptionFunc<T>>) {
        self.publisher.lock().unwrap().delete_observer(subscription);
    }

    fn result(&mut self) -> Option<T> {
        self.result.lock().unwrap().clone()
    }
}

#[cfg(all(feature = "for_futures", feature = "publisher", feature = "handler"))]
impl<T> Future for WillAsync<T>
where
    T: Clone + Send + Sync + 'static,
{
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let started_alive = self.started_alive.lock().unwrap();
        let &(ref started, ref alive) = &*started_alive;

        if started.load(Ordering::SeqCst) && (!alive.load(Ordering::SeqCst)) {
            Poll::Ready(self.clone().result())
        } else {
            {
                self.waker.lock().unwrap().replace(cx.waker().clone());
            }
            Poll::Pending
        }
    }
}

/**
`CountDownLatch` implements a latch with a value(> 0),
waiting for the value counted down until <= 0
(the countdown action would be in other threads).

# Remarks

It's inspired by `CountDownLatch` in `Java`
, and easily use it on async scenaios.

``
*/
#[derive(Debug, Clone)]
pub struct CountDownLatch {
    pair: Arc<(Arc<Mutex<u64>>, Condvar)>,

    #[cfg(feature = "for_futures")]
    waker: Arc<Mutex<Option<Waker>>>,
}

impl CountDownLatch {
    pub fn new(count: u64) -> CountDownLatch {
        CountDownLatch {
            pair: Arc::new((Arc::new(Mutex::new(count)), Condvar::new())),

            #[cfg(feature = "for_futures")]
            waker: Arc::new(Mutex::new(None)),
        }
    }

    pub fn countdown(&self) {
        {
            let &(ref lock, ref cvar) = &*self.pair.clone();
            let mut started = lock.lock().unwrap();
            if *started > 0 {
                *started -= 1;
            }
            cvar.notify_one();

            #[cfg(feature = "for_futures")]
            {
                let mut waker = self.waker.lock().unwrap();
                if let Some(waker) = waker.take() {
                    waker.wake()
                }
            }
        }
    }

    pub fn wait(&self) {
        let &(ref lock, ref cvar) = &*self.pair;

        /*
        let mut result = lock.lock();
        let mut started;
        if result.is_err() {
            started = result.err().unwrap().into_inner();
        } else {
            started = result.unwrap();
        }
        */
        let mut started = lock.lock().unwrap();

        while *started > 0 {
            let result = cvar.wait(started);

            if result.is_err() {
                started = result.err().unwrap().into_inner();
            } else {
                started = result.unwrap();
            }
        }
    }
}

#[cfg(feature = "for_futures")]
impl Future for CountDownLatch {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let &(ref remaining, _) = &*self.pair;
        let count = remaining.lock().unwrap();
        if *count > 0 {
            {
                self.waker.lock().unwrap().replace(cx.waker().clone());
            }
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

/**
`Queue` `trait` defined the interface which perform basic `Queue` actions.

# Arguments

* `T` - The generic type of data

# Remarks

It's inspired by `Queue` in `Java`.

``
*/
pub trait Queue<T> {
    fn offer(&mut self, v: T);
    fn poll(&mut self) -> Option<T>;
    fn put(&mut self, v: T);
    fn take(&mut self) -> Option<T>;
}

/**
`BlockingQueue` implements `Queue` `trait` and provides `BlockingQueue` features.

# Arguments

* `T` - The generic type of data

# Remarks

It's inspired by `BlockingQueue` in `Java`,
, and easily use it on async scenaios.

``
*/
#[derive(Debug, Clone)]
pub struct BlockingQueue<T> {
    pub timeout: Option<Duration>,
    pub panic: bool,
    alive: Arc<Mutex<AtomicBool>>,
    blocking_sender: Arc<Mutex<mpsc::Sender<T>>>,
    blocking_recever: Arc<Mutex<mpsc::Receiver<T>>>,
}

// impl <T> Copy for BlockingQueue<T> {
//     fn clone(&self) -> BlockingQueue<T> {
//         *self
//     }
// }

impl<T> Default for BlockingQueue<T> {
    fn default() -> Self {
        let (blocking_sender, blocking_recever) = mpsc::channel();

        BlockingQueue {
            alive: Arc::new(Mutex::new(AtomicBool::new(true))),
            timeout: None,
            panic: false,
            blocking_sender: Arc::new(Mutex::new(blocking_sender)),
            blocking_recever: Arc::new(Mutex::new(blocking_recever)),
        }
    }
}

impl<T> BlockingQueue<T> {
    pub fn new() -> BlockingQueue<T> {
        Default::default()
    }

    pub fn is_alive(&self) -> bool {
        let alive = &self.alive.lock().unwrap();
        alive.load(Ordering::SeqCst)
    }

    pub fn stop(&mut self) {
        {
            let alive = &self.alive.lock().unwrap();
            if !alive.load(Ordering::SeqCst) {
                return;
            }
            alive.store(false, Ordering::SeqCst);

            let sender = self.blocking_sender.lock().unwrap();
            drop(sender);
        }
    }
}

impl<T> Queue<T> for BlockingQueue<T>
where
    T: 'static + Send,
{
    fn offer(&mut self, v: T) {
        {
            let alive = &self.alive.lock().unwrap();
            if !alive.load(Ordering::SeqCst) {
                return;
            }

            let result = self.blocking_sender.lock().unwrap().send(v);
            if self.panic && result.is_err() {
                std::panic::panic_any(result.err());
            }
        }
    }

    fn poll(&mut self) -> Option<T> {
        let result = self.poll_result();

        if self.panic && result.is_err() {
            std::panic::panic_any(result.err());
            // return None;
        }

        match result {
            Ok(v) => Some(v),
            Err(_) => None,
        }
    }

    fn put(&mut self, v: T) {
        // NOTE Currently there's no maximum size of BlockingQueue.

        self.offer(v);
    }

    fn take(&mut self) -> Option<T> {
        let result = self.take_result();

        if self.panic && result.is_err() {
            std::panic::panic_any(result.err());
            // return None;
        }

        match result {
            Ok(v) => Some(v),
            Err(_) => None,
        }
    }
}

impl<T> BlockingQueue<T>
where
    T: Send + 'static,
{
    pub fn poll_result(&mut self) -> Result<T, Box<dyn Error + Send>> {
        if !self.is_alive() {
            return Err(Box::new(RecvTimeoutError::Disconnected));
        }

        {
            let result = { self.blocking_recever.lock().unwrap().try_recv() };

            match result {
                Ok(v) => Ok(v),
                Err(e) => Err(Box::new(e)),
            }
        }
    }

    pub fn take_result(&mut self) -> Result<T, Box<dyn Error + Send>> {
        if !self.is_alive() {
            return Err(Box::new(RecvTimeoutError::Disconnected));
        }

        {
            match self.timeout {
                Some(duration) => {
                    let result = { self.blocking_recever.lock().unwrap().recv_timeout(duration) };

                    match result {
                        Ok(v) => Ok(v),
                        Err(e) => Err(Box::new(e)),
                    }
                }
                None => {
                    let result = { self.blocking_recever.lock().unwrap().recv() };

                    match result {
                        Ok(v) => Ok(v),
                        Err(e) => Err(Box::new(e)),
                    }
                }
            }
        }
    }
}
#[cfg(feature = "for_futures")]
impl<T> BlockingQueue<T>
where
    T: Send + 'static + Clone,
{
    pub async fn poll_result_as_future(&mut self) -> Result<T, Box<dyn Error + Send>> {
        let mut queue = self.clone();

        let spawn_future_result = {
            shared_thread_pool()
                .inner
                .lock()
                .unwrap()
                .spawn_with_handle(async move { queue.poll_result() })
        };
        match spawn_future_result {
            Ok(future) => future.await,
            Err(e) => Err(Box::new(e)),
        }
    }
    pub async fn take_result_as_future(&mut self) -> Result<T, Box<dyn Error + Send>> {
        let mut queue = self.clone();

        let spawn_future_result = {
            shared_thread_pool()
                .inner
                .lock()
                .unwrap()
                .spawn_with_handle(async move { queue.take_result() })
        };
        match spawn_future_result {
            Ok(future) => future.await,
            Err(e) => Err(Box::new(e)),
        }
    }
}

#[cfg(feature = "for_futures")]
#[futures_test::test]
async fn test_sync_future() {
    let mut wa = WillAsync::new(move || 1);
    wa.start();

    assert_eq!(Some(1), wa.await);

    let mut _h = HandlerThread::new_with_mutex();
    let mut pub1 = Publisher::new_with_handlers(Some(_h.clone()));

    let latch = CountDownLatch::new(4);
    let latch2 = latch.clone();

    let _ = pub1.subscribe(Arc::new(SubscriptionFunc::new(move |y| {
        println!("test_sync_future {:?}", y);
        latch2.countdown();
    })));

    println!("test_sync_future before Publisher.start()");

    {
        let h = &mut _h.lock().unwrap();

        println!("test_sync_future hh2");
        h.start();
        println!("test_sync_future hh2 running");
    }
    std::thread::sleep(Duration::from_millis(1));

    pub1.publish(1);
    println!("test_sync_future pub1.publish");
    pub1.publish(2);
    println!("test_sync_future pub1.publish");
    pub1.publish(3);
    println!("test_sync_future pub1.publish");
    pub1.publish(4);
    println!("test_sync_future pub1.publish");

    let _ = latch.await;
    println!("test_sync_future done");
}

#[test]
fn test_will_sync_new() {
    use std::thread;
    use std::time;

    let latch = CountDownLatch::new(1);
    let latch2 = latch.clone();
    let mut h = WillAsync::new(move || 1);
    assert_eq!(false, h.is_alive());
    assert_eq!(false, h.is_started());
    h.stop();
    h.stop();
    assert_eq!(false, h.is_alive());
    assert_eq!(false, h.is_started());
    h.add_callback(Arc::new(SubscriptionFunc::new(move |_v: Arc<i16>| {
        assert_eq!(1, *Arc::make_mut(&mut _v.clone()));
        latch2.countdown();
    })));
    h.start();
    latch.clone().wait();
    thread::sleep(time::Duration::from_millis(1));
    assert_eq!(false, h.is_alive());
    assert_eq!(true, h.is_started());
    assert_eq!(1, h.result().unwrap());
}

#[test]
fn test_sync_countdownlatch_zero_does_not_block() {
    // A latch initialized at 0 is already released.
    let latch = CountDownLatch::new(0);
    latch.wait();
}

#[test]
fn test_sync_countdownlatch_countdown_releases() {
    let latch = CountDownLatch::new(2);
    latch.countdown();
    latch.countdown();
    // Now at zero; wait returns.
    latch.wait();
}

#[test]
fn test_sync_countdownlatch_countdown_below_zero_is_safe() {
    // Extra countdowns past zero are guarded and do not underflow.
    let latch = CountDownLatch::new(1);
    latch.countdown();
    latch.countdown();
    latch.countdown();
    latch.wait();
}

#[test]
fn test_sync_countdownlatch_cross_thread() {
    use std::thread;

    let latch = CountDownLatch::new(5);
    let latch_thread = latch.clone();
    thread::spawn(move || {
        for _ in 0..5 {
            latch_thread.countdown();
        }
    });
    // Blocks until the worker thread finishes counting down.
    latch.wait();
}

#[test]
fn test_sync_blocking_queue_fifo_offer_poll() {
    let mut q = BlockingQueue::<i32>::new();
    q.offer(1);
    q.offer(2);
    q.offer(3);
    assert_eq!(Some(1), q.poll());
    assert_eq!(Some(2), q.poll());
    assert_eq!(Some(3), q.poll());
}

#[test]
fn test_sync_blocking_queue_poll_empty_is_none() {
    // poll() is non-blocking; an empty queue yields None immediately.
    let mut q = BlockingQueue::<i32>::new();
    assert_eq!(None, q.poll());
}

#[test]
fn test_sync_blocking_queue_put_take_fifo() {
    let mut q = BlockingQueue::<i32>::new();
    q.put(10);
    q.put(20);
    assert_eq!(Some(10), q.take());
    assert_eq!(Some(20), q.take());
}

#[test]
fn test_sync_blocking_queue_take_timeout_is_none() {
    // With a timeout set, take() on an empty queue gives up and returns None.
    let mut q = BlockingQueue::<i32>::new();
    q.timeout = Some(Duration::from_millis(5));
    assert_eq!(None, q.take());
}

#[test]
fn test_sync_blocking_queue_is_alive_and_stop() {
    let mut q = BlockingQueue::<i32>::new();
    assert_eq!(true, q.is_alive());

    q.stop();
    assert_eq!(false, q.is_alive());

    // After stop, offer is a no-op and poll/take return None.
    q.offer(1);
    assert_eq!(None, q.poll());
    assert_eq!(None, q.take());

    // Stopping again is idempotent.
    q.stop();
    assert_eq!(false, q.is_alive());
}

#[test]
fn test_sync_blocking_queue_poll_result_ok_and_err() {
    let mut q = BlockingQueue::<i32>::new();
    q.offer(7);
    assert_eq!(true, q.poll_result().is_ok());
    // Now empty: try_recv errors out.
    assert_eq!(true, q.poll_result().is_err());
}

#[test]
fn test_sync_blocking_queue_cross_thread_producer() {
    use std::thread;

    let mut q = BlockingQueue::<i32>::new();
    // Guard against hangs: bound the blocking take.
    q.timeout = Some(Duration::from_secs(5));
    let mut producer = q.clone();
    thread::spawn(move || {
        producer.put(42);
    });
    // take() blocks until the producer enqueues.
    assert_eq!(Some(42), q.take());
}

#[test]
fn test_sync_blocking_queue_default_matches_new() {
    let mut a = BlockingQueue::<i32>::new();
    let mut b: BlockingQueue<i32> = Default::default();
    assert_eq!(true, a.is_alive());
    assert_eq!(true, b.is_alive());
    a.offer(1);
    b.offer(2);
    assert_eq!(Some(1), a.poll());
    assert_eq!(Some(2), b.poll());
}

#[test]
fn test_sync_will_async_lifecycle_and_result() {
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    let latch = CountDownLatch::new(1);
    let latch_thread = latch.clone();
    let observed = Arc::new(AtomicI32::new(0));
    let observed_thread = observed.clone();

    let mut w = WillAsync::new(move || 21 * 2);

    // Before start: neither started nor alive.
    assert_eq!(false, w.is_started());
    assert_eq!(false, w.is_alive());
    // stop before start is a no-op.
    w.stop();
    assert_eq!(false, w.is_started());

    w.add_callback(Arc::new(SubscriptionFunc::new(move |v: Arc<i32>| {
        observed_thread.store(*v, Ordering::SeqCst);
        latch_thread.countdown();
    })));

    w.start();
    latch.wait();

    assert_eq!(42, observed.load(Ordering::SeqCst));
    assert_eq!(true, w.is_started());
    assert_eq!(Some(42), w.result());
}

#[test]
fn test_sync_will_async_remove_callback() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let fired = Arc::new(AtomicBool::new(false));
    let fired_thread = fired.clone();

    let mut w = WillAsync::new(move || 1);
    let sub = Arc::new(SubscriptionFunc::new(move |_v: Arc<i32>| {
        fired_thread.store(true, Ordering::SeqCst);
    }));
    w.add_callback(sub.clone());
    // Remove before starting so the callback must not fire.
    w.remove_callback(sub);

    let done = CountDownLatch::new(1);
    let done_thread = done.clone();
    w.add_callback(Arc::new(SubscriptionFunc::new(move |_v: Arc<i32>| {
        done_thread.countdown();
    })));
    w.start();
    done.wait();

    assert_eq!(false, fired.load(Ordering::SeqCst));
    assert_eq!(Some(1), w.result());
}

#[test]
fn test_sync_will_async_double_start_is_safe() {
    use std::sync::Arc;

    let latch = CountDownLatch::new(1);
    let latch_thread = latch.clone();
    let mut w = WillAsync::new(move || 5);
    w.add_callback(Arc::new(SubscriptionFunc::new(move |_v: Arc<i32>| {
        latch_thread.countdown();
    })));
    w.start();
    // Second start is ignored (already started).
    w.start();
    latch.wait();
    assert_eq!(Some(5), w.result());
}
