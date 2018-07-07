use std::{sync, thread, time};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use sync as fpSync;
use sync::{Queue};

pub trait Handler<Functor> {
    fn post(&mut self, func : Functor);
}

pub struct HandlerThread<Functor> {
    alive: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
    q: Arc<fpSync::BlockingQueue<Functor>>,
}

impl <Functor: 'static + FnOnce() + Send + Sync> HandlerThread<Functor> {
    pub fn new() -> HandlerThread<Functor> {
        let mut new_one = HandlerThread {
            handle: None,
            alive: Arc::new(AtomicBool::new(false)),
            q: Arc::new(<fpSync::BlockingQueue<Functor>>::new()),
        };

        return new_one;
    }

    pub fn start(mut self) {
        self.alive.store(true, Ordering::SeqCst);

        let alive = self.alive.clone();

        let mut _me = Arc::new(self);

        self.handle = Some(thread::spawn(move || {
            let me = &mut Arc::get_mut(&mut _me).unwrap();
            let q = Arc::get_mut(&mut me.q).unwrap();

            while alive.load(Ordering::SeqCst) {
                let v = q.take();

                v();
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
        let mut _me = Arc::new(self);
        let me = &mut Arc::get_mut(&mut _me).unwrap();

        let q = Arc::get_mut(&mut me.q).unwrap();
        q.put(func);
    }
}

#[test]
fn test_handler_new() {

    // let h = HandlerThread::new();
}
