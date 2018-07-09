use std::sync::{
    Arc,
    Mutex,

    mpsc,
};

pub trait Queue<T> {
    fn offer(&mut self, v : T);
    fn poll(&mut self) -> Option<T>;
    fn put(&mut self, v : T);
    fn take(&mut self) -> T;
}

#[derive(Debug, Clone)]
pub struct BlockingQueue<T> {
    blocking_sender: Arc<Mutex<mpsc::Sender<T>>>,
    blocking_recever: Arc<Mutex<mpsc::Receiver<T>>>,
}

// impl <T> Copy for BlockingQueue<T> {
//     fn clone(&self) -> BlockingQueue<T> {
//         *self
//     }
// }

impl <T> BlockingQueue<T> {
    pub fn new() -> BlockingQueue<T> {
        let (blocking_sender,blocking_recever) = mpsc::channel();

        return BlockingQueue {
            blocking_sender: Arc::new(Mutex::new(blocking_sender)),
            blocking_recever: Arc::new(Mutex::new(blocking_recever)),
        };
    }
}

impl <T> Queue<T> for BlockingQueue<T> {
    fn offer(&mut self, v : T) {
        {
            let _result = self.blocking_sender.lock().unwrap().send(v);
        }
    }

    fn poll(&mut self) -> Option<T> {
        let v : Option<T>;

        {
            let result = self.blocking_recever.lock().unwrap().recv();
            v = result.ok();
        }

        return v;
    }

    fn put(&mut self, v : T) {
        // NOTE Currently there's no maximum size of BlockingQueue.

        self.offer(v);
    }

    fn take(&mut self) -> T {
        loop {
            match self.poll() {
                Some(_x) => return _x,
                None => (),
            }
        }
    }
}
