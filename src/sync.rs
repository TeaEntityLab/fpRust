use std::sync::{
    Arc,
    Mutex,
    Condvar,

    mpsc,
};

pub struct BlockingQueue<T> {
    lock: Arc<Mutex<u16>>,
    condvar: Arc<Condvar>,
    blockingSender: mpsc::Sender<u16>,
    blockingRecever: mpsc::Receiver<u16>,

    queue: Vec<T>,
}

impl <T> BlockingQueue<T> {
    pub fn new() -> BlockingQueue<T> {
        let (blockingSender,blockingRecever) = mpsc::channel();

        return BlockingQueue {
            lock: Arc::new(Mutex::new(0_u16)),
            condvar: Arc::new(Condvar::new()),
            blockingSender,
            blockingRecever,

            queue: vec!(),
        };
    }

    pub fn offer(&mut self, mut v : T) {
        {
            let lock = self.lock.lock().unwrap();

            self.queue.push(v);
            self.blockingSender.send(0);
        }
    }

    pub fn poll(&mut self) -> Option<T> {
        let mut v : Option<T>;

        {
            let lock = self.lock.lock().unwrap();

            v = self.queue.pop();
        }

        return v;
    }

    pub fn put(&mut self, mut v : T) {
        // NOTE Currently there's no maximum size of BlockingQueue.

        self.offer(v);
    }

    pub fn take(&mut self) -> T {
        loop {
            match self.poll() {
                Some(_x) => return _x,
                None => (),
            }

            let okNext = self.blockingRecever.recv();
        }
    }
}
