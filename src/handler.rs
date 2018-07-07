use std::rc::Rc;

use std::{sync, thread, time};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use common::RawFunc;
use sync as fpSync;
use sync::{Queue};

pub trait Handler {
    fn post(&mut self, func : RawFunc);
}

#[derive(Clone)]
pub struct HandlerThread {
    // this: Option<Arc<HandlerThread>>,

    alive: Arc<AtomicBool>,
    handle: Arc<Option<thread::JoinHandle<()>>>,
    q: Arc<fpSync::BlockingQueue<RawFunc>>,
}

impl HandlerThread {
    pub fn new() -> HandlerThread {
        let mut new_one = HandlerThread {
            // this: None,

            handle: Arc::new(None),
            alive: Arc::new(AtomicBool::new(false)),
            q: Arc::new(<fpSync::BlockingQueue<RawFunc>>::new()),
        };

        return new_one;
    }

    pub fn start(&'static mut self) {
        self.alive.store(true, Ordering::SeqCst);
        let alive = self.alive.clone();

        // let mut myself = Arc::new(self);
        // let mut _me = myself.clone();
        // let me = Arc::get_mut(&mut _me).unwrap();
        // me.this = Some(myself.clone());

        let mut myself = Arc::new(self);
        let mut _me = myself.clone();
        let me = Arc::get_mut(&mut _me).unwrap();

        me.handle = Arc::new(Some(thread::spawn(move || {
            let mut _me = myself.clone();
            let me = Arc::get_mut(&mut _me).unwrap();
            // let q = Arc::get_mut(&mut me.q).unwrap();

            let mut _me = myself.clone();

            let q = Arc::get_mut(&mut me.q).unwrap();

            while alive.load(Ordering::SeqCst) {
                let v = q.take();

                v.invoke();
            }
        })));
    }

    pub fn stop(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
        let mut _handle = Box::new(&mut self.handle);
        let mut handle = Arc::get_mut(&mut _handle).unwrap();
        handle
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

    let mut __h = HandlerThread::new();
    let mut _h = Arc::new(__h);

    // let mut h1 : &'static mut HandlerThread = &mut _h.clone();
    // // Arc::get_mut(&mut h1).unwrap().start();
    // let mut h2 : &'static mut HandlerThread = &mut _h.clone();
    // Arc::get_mut(h2).unwrap().post(RawFunc::new(||{}));
}
