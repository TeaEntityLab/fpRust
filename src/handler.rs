use std::{sync, thread, time};
use std::sync::atomic::{AtomicBool, Ordering};

pub trait Handler {
    fn post(&mut self, func: impl FnOnce());
}

pub struct HandlerThread {
    alive: sync::Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl HandlerThread {
    pub fn new() -> HandlerThread {
        HandlerThread {
            handle: None,
            alive: sync::Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start<F>(&mut self, fun: F)
        where F: 'static + Send + FnMut() -> ()
    {
        self.alive.store(true, Ordering::SeqCst);

        let alive = self.alive.clone();

        self.handle = Some(thread::spawn(move || {
            let mut fun = fun;
            while alive.load(Ordering::SeqCst) {
                fun();
                thread::sleep(time::Duration::from_millis(10));
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
    fn post(&mut self, func: impl FnOnce()) {

    }
}
