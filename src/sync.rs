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
}
