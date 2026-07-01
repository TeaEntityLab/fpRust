//! Synchronization primitives: latches, queues, and deferred work (`Will`).
//!
//! # Crate features
//!
//! | Item | Required features |
//! |------|-------------------|
//! | Module (`sync`) | `sync` (part of default `pure`) |
//! | [`WillAsync`] | `publisher` **and** `handler` |
//! | [`WillAsync`] as `Future` | above + `for_futures` |
//! | [`CountDownLatch`] as `Future` | `for_futures` |
//! | `BlockingQueue::poll_result_as_future` / `BlockingQueue::take_result_as_future` | `for_futures` |
//!
//! Enable everything this module can expose in tests with the `test_runtime`
//! feature (`pure` + `for_futures`).
//!
//! # Shutdown and blocking behavior
//!
//! [`WillAsync::stop`] flips the alive flag only; it does not cancel an effect
//! already running on a [`Handler`]. A cooperative
//! shutdown protocol (join, interrupt, drain) is **not** implemented here and
//! remains a deferred design item.
//!
//! [`BlockingQueue::stop`] flips the alive flag so later `offer`/`take` are
//! rejected; it does **not** close the shared `mpsc` channel, so a consumer
//! already blocked in [`BlockingQueue::take`] stays blocked until its own
//! `timeout` fires (or every clone is dropped).

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

/// Deferred computation with completion callbacks (Java `Future`-like).
///
/// The primary implementation is [`WillAsync`] when both `publisher` and
/// `handler` features are enabled.
pub trait Will<T>: Send + Sync + 'static {
    /// Returns `true` after [`start`](Will::start) has been called, even if
    /// [`stop`](Will::stop) has already run.
    fn is_started(&mut self) -> bool;

    /// Returns `true` while the effect may still run (started and not stopped).
    fn is_alive(&mut self) -> bool;

    /// Schedules the effect on the backing [`Handler`].
    fn start(&mut self);

    /// Marks the will as no longer alive. Does not join or interrupt the handler
    /// thread; an in-flight effect may still finish.
    fn stop(&mut self);

    /// Registers a callback invoked when the result is published.
    fn add_callback(&mut self, subscription: Arc<SubscriptionFunc<T>>);

    /// Removes a previously registered callback.
    fn remove_callback(&mut self, subscription: Arc<SubscriptionFunc<T>>);

    /// Returns the last computed result, if any.
    fn result(&mut self) -> Option<T>;
}

/// Runs a `FnMut() -> T` on a [`HandlerThread`],
/// publishes the result through [`Publisher`], and
/// exposes [`Will`] lifecycle methods.
///
/// Requires **`publisher`** and **`handler`**. With **`for_futures`**, also
/// implements [`std::future::Future`] (`Output = Option<T>`).
#[cfg(all(feature = "publisher", feature = "handler"))]
#[derive(Clone)]
pub struct WillAsync<T> {
    effect: Arc<Mutex<dyn FnMut() -> T + Send + Sync + 'static>>,
    handler: Arc<Mutex<dyn Handler>>,
    publisher: Arc<Mutex<Publisher<T>>>,
    started_alive: Arc<Mutex<(AtomicBool, AtomicBool)>>,
    result: Arc<Mutex<Option<T>>>,

    #[cfg(feature = "for_futures")]
    waker: Arc<Mutex<Vec<Waker>>>,
}

#[cfg(all(feature = "publisher", feature = "handler"))]
impl<T> WillAsync<T> {
    /// Creates a will backed by a default [`HandlerThread`].
    pub fn new(effect: impl FnMut() -> T + Send + Sync + 'static) -> WillAsync<T> {
        Self::new_with_handler(effect, HandlerThread::new_with_mutex())
    }
    /// Creates a will that runs on the given handler.
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
            waker: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[cfg(feature = "for_futures")]
fn wake_all_wakers(wakers: &Arc<Mutex<Vec<Waker>>>) {
    // Drain under the lock, then wake AFTER releasing it: a waker may
    // synchronously re-poll on this thread and re-register, which would
    // self-deadlock the non-reentrant Mutex if we woke while still holding it.
    let drained: Vec<Waker> = {
        let mut guard = wakers.lock().unwrap();
        guard.drain(..).collect()
    };
    for waker in drained {
        waker.wake();
    }
}

#[cfg(all(feature = "publisher", feature = "handler"))]
impl<T> Will<T> for WillAsync<T>
where
    T: Clone + Send + Sync + 'static,
{
    fn is_started(&mut self) -> bool {
        let started_alive = self.started_alive.lock().unwrap();
        let (started, _) = &*started_alive;
        started.load(Ordering::SeqCst)
    }

    fn is_alive(&mut self) -> bool {
        let started_alive = self.started_alive.lock().unwrap();
        let (_, alive) = &*started_alive;
        alive.load(Ordering::SeqCst)
    }

    fn start(&mut self) {
        {
            let started_alive = self.started_alive.lock().unwrap();
            let (started, alive) = &*started_alive;
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

        #[cfg(feature = "for_futures")]
        wake_all_wakers(&self.waker);
    }

    fn stop(&mut self) {
        {
            let started_alive = self.started_alive.lock().unwrap();
            let (started, alive) = &*started_alive;
            if !started.load(Ordering::SeqCst) {
                return;
            }
            if !alive.load(Ordering::SeqCst) {
                return;
            }
            alive.store(false, Ordering::SeqCst);
        }

        #[cfg(feature = "for_futures")]
        wake_all_wakers(&self.waker);
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
        let (started, alive) = &*started_alive;

        if started.load(Ordering::SeqCst) && (!alive.load(Ordering::SeqCst)) {
            Poll::Ready(self.clone().result())
        } else {
            self.waker.lock().unwrap().push(cx.waker().clone());
            Poll::Pending
        }
    }
}

/// Count-down latch (Java `CountDownLatch`-like).
///
/// [`wait`](CountDownLatch::wait) blocks the current thread until the internal
/// counter reaches zero. With the **`for_futures`** feature, also implements
/// [`std::future::Future`] (`Output = ()`).
#[derive(Debug, Clone)]
pub struct CountDownLatch {
    pair: Arc<(Arc<Mutex<u64>>, Condvar)>,

    #[cfg(feature = "for_futures")]
    waker: Arc<Mutex<Vec<Waker>>>,
}

impl CountDownLatch {
    /// Creates a latch initialized to `count` (values `> 0` are typical).
    pub fn new(count: u64) -> CountDownLatch {
        CountDownLatch {
            pair: Arc::new((Arc::new(Mutex::new(count)), Condvar::new())),

            #[cfg(feature = "for_futures")]
            waker: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Decrements the counter (never below zero). When the counter reaches
    /// zero, wakes every blocking and async waiter.
    pub fn countdown(&self) {
        let (lock, cvar) = &*self.pair;
        let should_wake = {
            let mut started = lock.lock().unwrap();
            if *started > 0 {
                *started -= 1;
            }
            *started == 0
        };

        if should_wake {
            cvar.notify_all();

            #[cfg(feature = "for_futures")]
            wake_all_wakers(&self.waker);
        }
    }

    /// Blocks until the counter is zero.
    pub fn wait(&self) {
        let (lock, cvar) = &*self.pair;

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
            started = match cvar.wait(started) {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
        }
    }
}

#[cfg(feature = "for_futures")]
impl Future for CountDownLatch {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let (remaining, _) = &*self.pair;
        let count = remaining.lock().unwrap();
        if *count > 0 {
            self.waker.lock().unwrap().push(cx.waker().clone());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

/// Minimal queue interface (Java `Queue`-like).
pub trait Queue<T> {
    /// Enqueues if the queue is alive (non-blocking for [`BlockingQueue`]).
    fn offer(&mut self, v: T);
    /// Non-blocking dequeue.
    fn poll(&mut self) -> Option<T>;
    /// Enqueues (alias of [`offer`](Queue::offer) for [`BlockingQueue`]).
    fn put(&mut self, v: T);
    /// Blocking dequeue when no timeout is configured on [`BlockingQueue`].
    fn take(&mut self) -> Option<T>;
}

/// Thread-safe **unbounded** channel wrapper with Java `BlockingQueue`-like
/// blocking `take`. Note: unlike a typical bounded Java `BlockingQueue`, `put`/`offer`
/// never block on capacity — the backing `mpsc::channel()` has no maximum size.
///
/// # Fields
///
/// * `timeout` — when set, [`take`](BlockingQueue::take) uses
///   [`recv_timeout`](std::sync::mpsc::Receiver::recv_timeout) instead of blocking forever.
/// * `panic` — when `true`, send/receive errors panic instead of mapping to `None`.
///
/// With **`for_futures`**, `BlockingQueue::poll_result_as_future` and `BlockingQueue::take_result_as_future`
/// offload blocking recv to the shared thread pool.
#[derive(Debug, Clone)]
pub struct BlockingQueue<T> {
    /// Optional upper bound on blocking [`take`](BlockingQueue::take) waits.
    pub timeout: Option<Duration>,
    /// When `true`, propagate channel errors as panics.
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
    /// Creates an open queue backed by an `mpsc` channel.
    pub fn new() -> BlockingQueue<T> {
        Default::default()
    }

    /// Returns whether the queue accepts new items.
    pub fn is_alive(&self) -> bool {
        let alive = &self.alive.lock().unwrap();
        alive.load(Ordering::SeqCst)
    }

    /// Closes the queue: flips the `alive` flag so subsequent
    /// [`offer`](Queue::offer)/[`put`](Queue::put) are dropped and
    /// [`poll_result`](BlockingQueue::poll_result)/[`take_result`](BlockingQueue::take_result)
    /// return `Err(Disconnected)`.
    ///
    /// Does **not** join or interrupt a consumer already blocked inside
    /// [`take`](BlockingQueue::take): the shared `mpsc::Sender` lives behind
    /// `Arc<Mutex<..>>` and cannot be dropped here, so an in-flight `recv`
    /// only unblocks on its own `timeout` or when every clone is gone.
    pub fn stop(&mut self) {
        let alive = &self.alive.lock().unwrap();
        if !alive.load(Ordering::SeqCst) {
            return;
        }
        alive.store(false, Ordering::SeqCst);
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

        result.ok()
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

        result.ok()
    }
}

impl<T> BlockingQueue<T>
where
    T: Send + 'static,
{
    /// Non-blocking receive with explicit error type.
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

    /// Blocking receive honoring [`timeout`](BlockingQueue::timeout).
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
    /// **`for_futures` only:** non-blocking poll on the shared thread pool.
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
    /// **`for_futures` only:** blocking take on the shared thread pool.
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

#[cfg(test)]
#[cfg(feature = "for_futures")]
mod sync_future_tests {
    use super::*;
    use futures::executor::block_on;
    use futures::future::join;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    const TEST_DEADLINE: Duration = Duration::from_secs(5);

    fn block_on_with_deadline<F, T>(future: F, deadline: Duration) -> T
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let result = block_on(future);
            let _ = tx.send(result);
        });
        let end = Instant::now() + deadline;
        loop {
            match rx.try_recv() {
                Ok(value) => return value,
                Err(mpsc::TryRecvError::Empty) => {
                    assert!(
                        Instant::now() < end,
                        "async test timed out after {:?}",
                        deadline
                    );
                    std::thread::yield_now();
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    panic!("async worker thread panicked or dropped the result sender");
                }
            }
        }
    }

    #[futures_test::test]
    async fn test_will_async_multi_waker() {
        let mut wa = WillAsync::new(|| 99);
        let wa1 = wa.clone();
        let wa2 = wa.clone();

        let (tx, rx) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let joined = block_on(join(wa1, wa2));
            let _ = tx.send(joined);
        });

        for _ in 0..64 {
            std::thread::yield_now();
        }
        wa.start();

        let (r1, r2) = {
            let end = Instant::now() + TEST_DEADLINE;
            loop {
                match rx.try_recv() {
                    Ok(value) => break value,
                    Err(mpsc::TryRecvError::Empty) => {
                        assert!(Instant::now() < end, "WillAsync multi-waker test timed out");
                        std::thread::yield_now();
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        panic!("WillAsync multi-waker worker thread failed");
                    }
                }
            }
        };

        assert_eq!(Some(99), r1);
        assert_eq!(Some(99), r2);
    }

    #[futures_test::test]
    async fn test_will_async_poll_before_start() {
        let mut wa = WillAsync::new(|| 7);
        let noop = futures::task::noop_waker();
        let mut cx = Context::from_waker(&noop);
        assert!(matches!(Pin::new(&mut wa).poll(&mut cx), Poll::Pending));

        wa.start();

        let result = block_on_with_deadline(wa, TEST_DEADLINE);
        assert_eq!(Some(7), result);
    }

    #[futures_test::test]
    async fn test_countdownlatch_multi_waker() {
        let latch = CountDownLatch::new(2);
        let latch1 = latch.clone();
        let latch2 = latch.clone();

        let (tx, rx) = mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let joined = block_on(join(latch1, latch2));
            let _ = tx.send(joined);
        });

        for _ in 0..64 {
            std::thread::yield_now();
        }

        std::thread::spawn(move || {
            latch.countdown();
            latch.countdown();
        });

        let ((), ()) = {
            let end = Instant::now() + TEST_DEADLINE;
            loop {
                match rx.try_recv() {
                    Ok(value) => break value,
                    Err(mpsc::TryRecvError::Empty) => {
                        assert!(
                            Instant::now() < end,
                            "CountDownLatch multi-waker test timed out"
                        );
                        std::thread::yield_now();
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        panic!("CountDownLatch multi-waker worker thread failed");
                    }
                }
            }
        };
    }

    #[futures_test::test]
    async fn test_countdownlatch_poll_edges() {
        let latch = CountDownLatch::new(1);
        let mut latch_for_poll = latch.clone();
        let noop = futures::task::noop_waker();
        let mut cx = Context::from_waker(&noop);

        assert!(matches!(
            Pin::new(&mut latch_for_poll).poll(&mut cx),
            Poll::Pending
        ));

        latch.countdown();

        assert!(matches!(
            Pin::new(&mut latch_for_poll).poll(&mut cx),
            Poll::Ready(())
        ));
    }

    #[futures_test::test]
    async fn test_blocking_queue_take_result_as_future_ok() {
        let mut queue = BlockingQueue::<i32>::new();
        queue.offer(42);

        let result = block_on_with_deadline(
            async move { queue.take_result_as_future().await },
            TEST_DEADLINE,
        );
        assert!(matches!(result, Ok(42)));
    }

    #[futures_test::test]
    async fn test_blocking_queue_take_result_as_future_empty_timeout_err() {
        let mut queue = BlockingQueue::<i32>::new();
        queue.timeout = Some(Duration::from_millis(50));

        let result = block_on_with_deadline(
            async move { queue.take_result_as_future().await },
            TEST_DEADLINE,
        );
        assert!(result.is_err());
    }

    #[futures_test::test]
    async fn test_blocking_queue_take_result_as_future_stopped_err() {
        let mut queue = BlockingQueue::<i32>::new();
        queue.stop();

        let result = block_on_with_deadline(
            async move { queue.take_result_as_future().await },
            TEST_DEADLINE,
        );
        assert!(result.is_err());
    }

    #[futures_test::test]
    async fn test_blocking_queue_poll_result_as_future_ok() {
        let mut queue = BlockingQueue::<&'static str>::new();
        queue.offer("value");

        let result = block_on_with_deadline(
            async move { queue.poll_result_as_future().await },
            TEST_DEADLINE,
        );
        assert!(matches!(result, Ok("value")));
    }

    #[futures_test::test]
    async fn test_blocking_queue_poll_result_as_future_empty_err() {
        let mut queue = BlockingQueue::<i32>::new();

        let result = block_on_with_deadline(
            async move { queue.poll_result_as_future().await },
            TEST_DEADLINE,
        );
        assert!(result.is_err());
    }

    #[futures_test::test]
    async fn test_blocking_queue_poll_result_as_future_stopped_err() {
        let mut queue = BlockingQueue::<i32>::new();
        queue.stop();

        let result = block_on_with_deadline(
            async move { queue.poll_result_as_future().await },
            TEST_DEADLINE,
        );
        assert!(result.is_err());
    }
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
    // The latch fires inside publish() (one statement before this.stop() flips
    // alive=false on the handler thread), so wait for that flip deterministically
    // instead of racing a fixed sleep. The 5s ceiling only bounds a genuine hang.
    let alive_deadline = time::Instant::now() + time::Duration::from_secs(5);
    while h.is_alive() && time::Instant::now() < alive_deadline {
        thread::yield_now();
    }
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
fn test_sync_countdownlatch_releases_all_waiters() {
    use std::sync::mpsc;
    use std::thread;

    let latch = CountDownLatch::new(1);
    let (ready_tx, ready_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let mut handles = Vec::new();

    for _ in 0..4 {
        let waiter = latch.clone();
        let ready_tx = ready_tx.clone();
        let done_tx = done_tx.clone();
        handles.push(thread::spawn(move || {
            ready_tx.send(()).unwrap();
            waiter.wait();
            done_tx.send(()).unwrap();
        }));
    }
    drop(ready_tx);
    drop(done_tx);

    for _ in 0..4 {
        assert_eq!(true, ready_rx.recv_timeout(Duration::from_secs(1)).is_ok());
    }
    for _ in 0..100 {
        thread::yield_now();
    }

    latch.countdown();

    for _ in 0..4 {
        assert_eq!(true, done_rx.recv_timeout(Duration::from_secs(1)).is_ok());
    }
    for handle in handles {
        handle.join().unwrap();
    }
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
