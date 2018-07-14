/*!
In this module there're implementations & tests
of general async handling features.
*/

use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Condvar, Mutex,
};
use std::time::Duration;

/**
`CountDownLatch` implements a latch with a value(> 0),
waiting for the value counted down until <= 0
(the countdown action would be in other threads).

# Remarks

It's inspired by `CountDownLatch` in `Java`
, and easily use it on async scenaios.

``
*/
#[derive(Debug, Clone)]
pub struct CountDownLatch {
    pair: Arc<(Arc<Mutex<u64>>, Condvar)>,
}

impl CountDownLatch {
    pub fn new(count: u64) -> CountDownLatch {
        return CountDownLatch {
            pair: Arc::new((Arc::new(Mutex::new(count)), Condvar::new())),
        };
    }

    pub fn countdown(&self) {
        let &(ref lock, ref cvar) = &*self.pair.clone();
        let mut started = lock.lock().unwrap();
        if *started > 0 {
            *started -= 1;
        }
        cvar.notify_one();
    }

    pub fn wait(&self) {
        let &(ref lock, ref cvar) = &*self.pair.clone();

        /*
        let mut result = lock.lock();
        let mut started;
        if result.is_err() {
            started = result.err().unwrap().into_inner();
        } else {
            started = result.unwrap();
        }
        */
        let mut started = lock.lock().unwrap();

        while *started > 0 {
            let result = cvar.wait(started);

            if result.is_err() {
                started = result.err().unwrap().into_inner();
            } else {
                started = result.unwrap();
            }
        }
    }
}

/**
`Queue` `trait` defined the interface which perform basic `Queue` actions.

# Arguments

* `T` - The generic type of data

# Remarks

It's inspired by `Queue` in `Java`.

``
*/
pub trait Queue<T> {
    fn offer(&mut self, v: T);
    fn poll(&mut self) -> Option<T>;
    fn put(&mut self, v: T);
    fn take(&mut self) -> Option<T>;
}

/**
`BlockingQueue` implements `Queue` `trait` and provides `BlockingQueue` features.

# Arguments

* `T` - The generic type of data

# Remarks

It's inspired by `BlockingQueue` in `Java`,
, and easily use it on async scenaios.

``
*/
#[derive(Debug, Clone)]
pub struct BlockingQueue<T> {
    pub timeout: Option<Duration>,
    pub panic: bool,
    alive: Arc<Mutex<AtomicBool>>,
    blocking_sender: Arc<Mutex<mpsc::Sender<T>>>,
    blocking_recever: Arc<Mutex<mpsc::Receiver<T>>>,
}

// impl <T> Copy for BlockingQueue<T> {
//     fn clone(&self) -> BlockingQueue<T> {
//         *self
//     }
// }

impl<T> BlockingQueue<T> {
    pub fn new() -> BlockingQueue<T> {
        let (blocking_sender, blocking_recever) = mpsc::channel();

        return BlockingQueue {
            alive: Arc::new(Mutex::new(AtomicBool::new(true))),
            timeout: None,
            panic: false,
            blocking_sender: Arc::new(Mutex::new(blocking_sender)),
            blocking_recever: Arc::new(Mutex::new(blocking_recever)),
        };
    }

    pub fn is_alive(&mut self) -> bool {
        let alive = &self.alive.lock().unwrap();
        return alive.load(Ordering::SeqCst);
    }

    pub fn stop(&mut self) {
        {
            let alive = &self.alive.lock().unwrap();
            if !alive.load(Ordering::SeqCst) {
                return;
            }
            alive.store(false, Ordering::SeqCst);

            let sender = self.blocking_sender.lock().unwrap();
            drop(sender);
        }
    }
}

impl<T: 'static + Send> Queue<T> for BlockingQueue<T> {
    fn offer(&mut self, v: T) {
        {
            let alive = &self.alive.lock().unwrap();
            if !alive.load(Ordering::SeqCst) {
                return;
            }

            let result = self.blocking_sender.lock().unwrap().send(v);
            if self.panic && result.is_err() {
                panic!(result.err());
            }
        }
    }

    fn poll(&mut self) -> Option<T> {
        if !self.is_alive() {
            return None::<T>;
        }

        {
            let result = self.blocking_recever.lock().unwrap().try_recv();

            if self.panic && result.is_err() {
                panic!(result.err());
            }

            return result.ok();
        }
    }

    fn put(&mut self, v: T) {
        // NOTE Currently there's no maximum size of BlockingQueue.

        self.offer(v);
    }

    fn take(&mut self) -> Option<T> {
        loop {
            if !self.is_alive() {
                return None::<T>;
            }

            {
                match self.timeout {
                    Some(duration) => {
                        let result = self.blocking_recever.lock().unwrap().recv_timeout(duration);

                        if self.panic && result.is_err() {
                            panic!(result.err());
                        }

                        return result.ok();
                    }
                    None => {
                        let result = self.blocking_recever.lock().unwrap().recv();

                        if self.panic && result.is_err() {
                            panic!(result.err());
                        }

                        return result.ok();
                    }
                }
            }
        }
    }
}
