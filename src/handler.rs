use std::rc::Rc;

use std::{sync, thread, time};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use common::RawFunc;
use sync as fpSync;
use sync::{Queue};

pub trait Handler {
    fn is_started(&mut self) -> bool;
    fn is_alive(&mut self) -> bool;

    fn start(&mut self);
    fn stop(&mut self);

    fn post(&mut self, func : RawFunc);
}

#[derive(Clone)]
pub struct HandlerThread {
    inner: Arc<HandlerThreadInner>,

    handle: Arc<Option<thread::JoinHandle<()>>>,
}

impl HandlerThread {
    pub fn new() -> HandlerThread {
        return HandlerThread {
            inner: Arc::new(HandlerThreadInner::new()),

            handle: Arc::new(None),
        };
    }
}

impl Handler for HandlerThread {

    fn is_started(&mut self) -> bool {
        let mut _inner = self.inner.clone();
        let inner = Arc::get_mut(&mut _inner).unwrap();

        return inner.is_started();
    }

    fn is_alive(&mut self) -> bool {
        let mut _inner = self.inner.clone();
        let inner = Arc::get_mut(&mut _inner).unwrap();

        return inner.is_alive();
    }

    fn start(&mut self) {

        let mut _inner = self.inner.clone();
        let inner = Arc::get_mut(&mut _inner).unwrap();
        if inner.is_started() {
            return;
        }

        let mut _inner_for_thread = self.inner.clone();
        self.handle = Arc::new(Some(thread::spawn(move || {
            let inner = Arc::get_mut(&mut _inner_for_thread).unwrap();
            inner.start();
        })));
    }

    fn stop(&mut self) {
        let mut _inner = self.inner.clone();
        let inner = Arc::get_mut(&mut _inner).unwrap();
        if !self.is_alive() {
            return;
        }
        inner.stop();

        let mut _handle = Box::new(&mut self.handle);
        let handle = Arc::get_mut(&mut _handle).unwrap();
        handle
            .take().expect("Called stop on non-running thread")
            .join().expect("Could not join spawned thread");
    }

    fn post(&mut self, func: RawFunc) {
        let mut _inner = self.inner.clone();
        let inner = Arc::get_mut(&mut _inner).unwrap();

        inner.post(func);
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
        return HandlerThreadInner {
            started: Arc::new(AtomicBool::new(false)),
            alive: Arc::new(AtomicBool::new(false)),
            q: Arc::new(<fpSync::BlockingQueue<RawFunc>>::new()),
        };
    }

}

impl Handler for HandlerThreadInner {

    fn is_started(&mut self) -> bool {
        return self.started.load(Ordering::SeqCst);
    }

    fn is_alive(&mut self) -> bool {
        return self.alive.load(Ordering::SeqCst);
    }

    fn start(&mut self){
        self.alive.store(true, Ordering::SeqCst);
        let alive = self.alive.clone();

        if self.is_started() {
            return;
        }
        self.started.store(true, Ordering::SeqCst);

        // let mut myself = Arc::new(self);
        // let mut _me = myself.clone();
        // let me = Arc::get_mut(&mut _me).unwrap();
        // me.this = Some(myself.clone());

        let myself = Arc::new(self);
        let mut _me = myself.clone();
        let me = Arc::get_mut(&mut _me).unwrap();

        let q = Arc::get_mut(&mut me.q).unwrap();

        while alive.load(Ordering::SeqCst) {
            let v = q.take();

            v.invoke();
        }
    }

    fn stop(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
    }

    fn post(&mut self, func: RawFunc) {
        let mut _me = Arc::new(self);
        let me = &mut Arc::get_mut(&mut _me).unwrap();

        let q = Arc::get_mut(&mut me.q).unwrap();
        q.put(func);
    }
}

#[test]
fn test_handler_new() {

    let mut _h = Box::new(HandlerThread::new());
    let mut h1 = _h.clone();
    h1.start();
    // Arc::get_mut(h2).unwrap().post(RawFunc::new(||{}));
}