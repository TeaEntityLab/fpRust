use std::rc::Rc;

use std::{sync, thread, time};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use common::RawFunc;
use sync as fpSync;
use sync::{Queue};

pub trait Handler {
    fn post(&mut self, func : RawFunc);
}

pub struct HandlerThread {
    alive: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
    q: Arc<fpSync::BlockingQueue<RawFunc>>,
}

impl HandlerThread {
    pub fn new() -> HandlerThread {
        let mut new_one = HandlerThread {
            handle: None,
            alive: Arc::new(AtomicBool::new(false)),
            q: Arc::new(<fpSync::BlockingQueue<RawFunc>>::new()),
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

                v.invoke();
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

impl Handler for HandlerThread {
    fn post(&mut self, func: RawFunc) {
        let mut _me = Arc::new(self);
        let me = &mut Arc::get_mut(&mut _me).unwrap();

        let q = Arc::get_mut(&mut me.q).unwrap();
        q.put(func);
    }
}

#[test]
fn test_handler_new() {

    // let mut h = &mut HandlerThread::new();
    // let mut _h = Rc::new(h);
    // (_h.clone()).start();
    // _h.clone().post(RawFunc::new(move || {
    //
    //     }));
}
