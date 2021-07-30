/*!
In this module there're implementations & tests of `Handler`.
(Inspired by `Android Handler`)
*/
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;

use super::common::RawFunc;
use super::sync as fpSync;
use super::sync::Queue;

/**
`Handler` `trait` defines the interface which could receive `FnMut` and run them on its own `thread`.

# Remarks

This is highly inspired by `Android Handler` concepts.

*/
pub trait Handler: Send + Sync + 'static {
    /**
    Did this `Handler` start?
    Return `true` when it did started (no matter it has stopped or not)

    */
    fn is_started(&mut self) -> bool;

    /**
    Is this `Handler` alive?
    Return `true` when it has started and not stopped yet.
    */
    fn is_alive(&mut self) -> bool;

    /**
    Start `Handler`.
    */
    fn start(&mut self);

    /**

    Stop `Cor`.
    This will make self.`is_alive`() returns `false`,
    and all `FnMut` posted by self.`post`() will not be executed.
    (Because it has stopped :P, that's reasonable)

    */
    fn stop(&mut self);

    /**
    Post a job which will run on this `Handler`.

    # Arguments

    * `func` - The posted job.
    ``
    */
    fn post(&mut self, func: RawFunc);
}

/**
`HandlerThread` could receive `FnMut` and run them on its own `thread`.
It implements `Handler` `trait` simply and works well.

This is kind of facade which just handles `thread`,
and bypasses jobs to `HandlerThreadInner`(private implementations).

# Remarks

This is highly inspired by `Android Handler` concepts.

*/
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
    pub fn new() -> HandlerThread {
        Default::default()
    }
    pub fn new_with_mutex() -> Arc<Mutex<HandlerThread>> {
        Arc::new(Mutex::new(HandlerThread::new()))
    }
}

impl Handler for HandlerThread {
    fn is_started(&mut self) -> bool {
        let _started_alive = self.started_alive.clone();
        let started_alive = _started_alive.lock().unwrap();
        let &(ref started, _) = &*started_alive;
        started.load(Ordering::SeqCst)
    }

    fn is_alive(&mut self) -> bool {
        let _started_alive = self.started_alive.clone();
        let started_alive = _started_alive.lock().unwrap();
        let &(_, ref alive) = &*started_alive;
        alive.load(Ordering::SeqCst)
    }

    fn start(&mut self) {
        {
            let _started_alive = self.started_alive.clone();
            let started_alive = _started_alive.lock().unwrap();
            let &(ref started, ref alive) = &*started_alive;

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
        let _started_alive = self.started_alive.clone();
        self.handle = Arc::new(Mutex::new(Some(thread::spawn(move || {
            Arc::make_mut(&mut _inner).start();

            this.stop();
        }))));
    }

    fn stop(&mut self) {
        {
            let _started_alive = self.started_alive.clone();
            let started_alive = _started_alive.lock().unwrap();
            let &(ref started, ref alive) = &*started_alive;

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
        let alive = self.alive.clone();

        if self.is_started() {
            return;
        }
        self.started.store(true, Ordering::SeqCst);

        let q = Arc::make_mut(&mut self.q);

        while alive.load(Ordering::SeqCst) {
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
    use std::time;

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

    thread::sleep(time::Duration::from_millis(50));

    assert_eq!(true, h.is_alive());
    assert_eq!(true, h.is_started());

    h.stop();
    thread::sleep(time::Duration::from_millis(50));

    assert_eq!(false, h.is_alive());
    assert_eq!(true, h.is_started());

    latch.clone().wait();
}
