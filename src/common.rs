use std::thread;

pub trait Subscription<X> {
    fn on_next(&mut self, x : X);
}

pub trait Handler {
    fn post(&mut self, func: impl FnOnce());
}

pub struct HandlerThread<'a> {
    cancel: bool,
    terminated: bool,
    th : &'a thread::Thread,
}

impl <'a> HandlerThread<'a> {
    fn new(th : &'a thread::Thread) -> HandlerThread {
        return HandlerThread {
            cancel: false,
            terminated: false,

            th: th,
        }
    }
}

impl <'a> Handler for HandlerThread<'a> {
    fn post(&mut self, func: impl FnOnce()) {

    }
}
