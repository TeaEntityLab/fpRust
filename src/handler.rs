use std::{sync, thread, time};
use std::sync::atomic::{AtomicBool, Ordering};

use sync as fpSync;

pub trait Handler<Functor> {
    fn post(&mut self, func : Functor);
}

pub struct HandlerThread<Functor> {
    // this: Option<sync::Arc<HandlerThread<Functor>>>,

    alive: sync::Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
    q: fpSync::BlockingQueue<Functor>,
}

impl <Functor : FnOnce() + Send + Sync> HandlerThread<Functor> {
    pub fn new() -> HandlerThread<Functor> {
        let mut newOne = HandlerThread {
            // this: None,

            handle: None,
            alive: sync::Arc::new(AtomicBool::new(false)),
            q: <fpSync::BlockingQueue<Functor>>::new(),
        };

        return newOne;
    }

    pub fn start(mut self) {
        self.alive.store(true, Ordering::SeqCst);

        let alive = self.alive.clone();

        // self.this = Some(sync::Arc::new(self));
        // let this = self.this.unwrap().clone();
        let this = sync::Arc::new(self);
        self.handle = Some(thread::spawn(move || {
            while alive.load(Ordering::SeqCst) {
                // let v = this.q.take();
            }
        }));
    }

    pub fn stop(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
        self.handle
            .take().expect("Called stop on non-running thread")
            .join().expect("Could not join spawned thread");
    }
}

impl <Functor : FnOnce()> Handler<Functor> for HandlerThread<Functor> {
    fn post(&mut self, func: Functor) {
        self.q.put(func);
    }
}

#[test]
fn test_handler_new() {

    // let h = HandlerThread::new();
}
