//! Single-threaded job queue inspired by Android `Handler` / `HandlerThread`.
//!
//! # Crate features
//!
//! Requires the **`handler`** feature (enabled by default `pure`). Depends on
//! **`sync`** ([`BlockingQueue`](crate::sync::BlockingQueue)).
//!
//! # Shutdown behavior
//!
//! [`Handler::stop`] and [`HandlerThread::stop`] clear the alive flag and stop
//! accepting new work. They do **not** [`join`](std::thread::JoinHandle::join) the
//! worker thread and do **not** interrupt a job already blocked inside
//! [`BlockingQueue::take`](crate::sync::BlockingQueue::take). Thread join-based
//! shutdown is intentionally commented out and remains a deferred design item.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;

use super::common::RawFunc;
use super::sync as fpSync;
use super::sync::Queue;

/// Runs posted [`RawFunc`] jobs sequentially on a dedicated thread.
pub trait Handler: Send + Sync + 'static {
    /// `true` after [`start`](Handler::start), even if [`stop`](Handler::stop) ran.
    fn is_started(&mut self) -> bool;

    /// `true` while the worker loop may still execute jobs.
    fn is_alive(&mut self) -> bool;

    /// Starts (or resumes) the worker loop.
    fn start(&mut self);

    /// Stops accepting new jobs. Does not join the worker thread or interrupt a
    /// blocked [`take`](crate::sync::BlockingQueue::take) inside the loop.
    fn stop(&mut self);

    /// Enqueues `func` for FIFO execution on the handler thread.
    fn post(&mut self, func: RawFunc);
}

/// [`Handler`] facade: spawns a thread whose inner loop drains a
/// [`BlockingQueue`](crate::sync::BlockingQueue) of [`RawFunc`] jobs.
#[derive(Clone)]
pub struct HandlerThread {
    started_alive: Arc<Mutex<(AtomicBool, AtomicBool)>>,

    inner: Arc<HandlerThreadInner>,

    handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

impl Default for HandlerThread {
    fn default() -> Self {
        HandlerThread {
            started_alive: Arc::new(Mutex::new((AtomicBool::new(false), AtomicBool::new(false)))),
            inner: Arc::new(HandlerThreadInner::new()),

            handle: Arc::new(Mutex::new(None)),
        }
    }
}

impl HandlerThread {
    /// Creates a handler that has not been started.
    pub fn new() -> HandlerThread {
        Default::default()
    }
    /// Wraps the handler for sharing across threads (typical for `Publisher` / `Will`).
    pub fn new_with_mutex() -> Arc<Mutex<HandlerThread>> {
        Arc::new(Mutex::new(HandlerThread::new()))
    }
}

impl Handler for HandlerThread {
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
        }

        let mut _inner = self.inner.clone();
        let mut this = self.clone();
        self.handle = Arc::new(Mutex::new(Some(thread::spawn(move || {
            Arc::make_mut(&mut _inner).start();

            this.stop();
        }))));
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

        Arc::make_mut(&mut self.inner).stop();

        // NOTE: Kill thread <- OS depending
        // let mut handle = self.handle.lock().unwrap();
        // handle
        //     .take()
        //     .expect("Called stop on non-running thread")
        //     .join()
        //     .expect("Could not join spawned thread");
    }

    fn post(&mut self, func: RawFunc) {
        Arc::make_mut(&mut self.inner).post(func);
    }
}

#[derive(Clone)]
struct HandlerThreadInner {
    // this: Option<Arc<HandlerThreadInner>>,
    started: Arc<AtomicBool>,
    alive: Arc<AtomicBool>,
    q: Arc<fpSync::BlockingQueue<RawFunc>>,
}

impl HandlerThreadInner {
    pub fn new() -> HandlerThreadInner {
        HandlerThreadInner {
            started: Arc::new(AtomicBool::new(false)),
            alive: Arc::new(AtomicBool::new(false)),
            q: Arc::new(<fpSync::BlockingQueue<RawFunc>>::new()),
        }
    }
}

impl Handler for HandlerThreadInner {
    fn is_started(&mut self) -> bool {
        self.started.load(Ordering::SeqCst)
    }

    fn is_alive(&mut self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    fn start(&mut self) {
        self.alive.store(true, Ordering::SeqCst);

        if self.is_started() {
            return;
        }
        self.started.store(true, Ordering::SeqCst);

        let q = Arc::make_mut(&mut self.q);

        while self.alive.load(Ordering::SeqCst) {
            let v = q.take();

            match v {
                Some(f) => {
                    f.invoke();
                }
                None => {
                    self.alive.store(false, Ordering::SeqCst);
                }
            }
        }
    }

    fn stop(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
    }

    fn post(&mut self, func: RawFunc) {
        let q = Arc::make_mut(&mut self.q);

        q.put(func);
    }
}

#[test]
fn test_handler_new() {
    use super::sync::CountDownLatch;

    let mut _h = HandlerThread::new_with_mutex();
    let mut h = _h.lock().unwrap();

    assert_eq!(false, h.is_alive());
    assert_eq!(false, h.is_started());

    h.stop();
    h.stop();
    assert_eq!(false, h.is_alive());
    assert_eq!(false, h.is_started());
    // let mut h1 = _h.clone();
    h.start();
    assert_eq!(true, h.is_alive());
    assert_eq!(true, h.is_started());

    let latch = CountDownLatch::new(1);
    let latch2 = latch.clone();

    // /*
    h.post(RawFunc::new(move || {
        println!("Executed !");

        let latch3 = latch2.clone();

        let mut _h2 = HandlerThread::new_with_mutex();
        let mut _h2_inside = _h2.clone();

        let mut h2 = _h2.lock().unwrap();
        h2.start();

        h2.post(RawFunc::new(move || {
            latch3.countdown();

            {
                _h2_inside.lock().unwrap().stop();
            }
        }));
    }));
    println!("Test");

    assert_eq!(true, h.is_alive());
    assert_eq!(true, h.is_started());

    h.stop();

    assert_eq!(false, h.is_alive());
    assert_eq!(true, h.is_started());

    latch.clone().wait();
}

#[test]
fn test_handler_lifecycle() {
    let mut h = HandlerThread::new();
    assert_eq!(false, h.is_started());
    assert_eq!(false, h.is_alive());

    h.start();
    assert_eq!(true, h.is_started());
    assert_eq!(true, h.is_alive());
}

#[test]
fn test_handler_stop_before_start_is_noop() {
    let mut h = HandlerThread::new();
    h.stop();
    assert_eq!(false, h.is_started());
    assert_eq!(false, h.is_alive());
}

#[test]
fn test_handler_post_executes_job() {
    use super::sync::CountDownLatch;
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    let value = Arc::new(AtomicI32::new(0));
    let value_thread = value.clone();
    let latch = CountDownLatch::new(1);
    let latch_thread = latch.clone();

    let mut h = HandlerThread::new();
    h.start();
    h.post(RawFunc::new(move || {
        value_thread.store(123, Ordering::SeqCst);
        latch_thread.countdown();
    }));

    latch.wait();
    assert_eq!(123, value.load(Ordering::SeqCst));
}

#[test]
fn test_handler_runs_jobs_in_fifo_order() {
    use super::sync::{BlockingQueue, CountDownLatch};

    let order = BlockingQueue::<i32>::new();
    let latch = CountDownLatch::new(1);
    let latch_thread = latch.clone();

    let mut h = HandlerThread::new();
    h.start();

    for i in 0..3 {
        let mut order_thread = order.clone();
        h.post(RawFunc::new(move || {
            order_thread.offer(i);
        }));
    }
    h.post(RawFunc::new(move || {
        latch_thread.countdown();
    }));

    latch.wait();
    let mut drained = order.clone();
    assert_eq!(Some(0), drained.poll());
    assert_eq!(Some(1), drained.poll());
    assert_eq!(Some(2), drained.poll());
}

#[test]
fn test_handler_double_start_keeps_running() {
    use super::sync::CountDownLatch;

    let latch = CountDownLatch::new(1);
    let latch_thread = latch.clone();

    let mut h = HandlerThread::new();
    h.start();
    // Second start must be ignored, not spawn a second loop.
    h.start();
    assert_eq!(true, h.is_alive());

    h.post(RawFunc::new(move || {
        latch_thread.countdown();
    }));
    latch.wait();
}
