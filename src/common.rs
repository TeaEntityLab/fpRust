use std::thread;

pub trait Subscription<X> {
    fn on_next(&mut self, x : X);
}

pub trait Handler {
    fn post(&mut self, func: impl FnOnce());
}

pub struct HandlerThread {
    cancel: bool,
    terminated: bool,
    th : &'static thread::Thread,
}

impl HandlerThread {
    // fn new() -> HandlerThread {
    //     let joinHandle = &thread::spawn(|| {
    //
    //     });
    //     let thread = joinHandle.thread();
    //
    //     return HandlerThread {
    //         cancel: false,
    //         terminated: false,
    //
    //         th: &thread,
    //     }
    // }
}

impl Handler for HandlerThread {
    fn post(&mut self, func: impl FnOnce()) {

    }
}
