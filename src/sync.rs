use std::sync::{
    Arc,
    Mutex,
    Condvar,

    mpsc,
};

pub trait Queue<T> {
    fn offer(&mut self, mut v : T);
    fn poll(&mut self) -> Option<T>;
    fn put(&mut self, mut v : T);
    fn take(&mut self) -> T;
}

pub struct BlockingQueue<T> {
    lock: Arc<Mutex<u16>>,
    condvar: Arc<Condvar>,
    blockingSender: Mutex<mpsc::Sender<u16>>,
    blockingRecever: Mutex<mpsc::Receiver<u16>>,

    queue: Vec<T>,
}

impl <T> BlockingQueue<T> {
    pub fn new() -> BlockingQueue<T> {
        let (blockingSender,blockingRecever) = mpsc::channel();

        return BlockingQueue {
            lock: Arc::new(Mutex::new(0_u16)),
            condvar: Arc::new(Condvar::new()),
            blockingSender: Mutex::new(blockingSender),
            blockingRecever: Mutex::new(blockingRecever),

            queue: vec!(),
        };
    }
}

impl <T> Queue<T> for BlockingQueue<T> {
    fn offer(&mut self, mut v : T) {
        {
            let lock = self.lock.lock().unwrap();

            self.queue.push(v);
            {
                self.blockingSender.lock().unwrap().send(0);
            }
        }
    }

    fn poll(&mut self) -> Option<T> {
        let mut v : Option<T>;

        {
            let lock = self.lock.lock().unwrap();

            v = self.queue.pop();
        }

        return v;
    }

    fn put(&mut self, mut v : T) {
        // NOTE Currently there's no maximum size of BlockingQueue.

        self.offer(v);
    }

    fn take(&mut self) -> T {
        loop {
            match self.poll() {
                Some(_x) => return _x,
                None => (),
            }

            {
                let okNext = self.blockingRecever.lock().unwrap().recv();
            }
        }
    }
}
