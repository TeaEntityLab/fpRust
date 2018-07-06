use std::sync::{
    Arc,
    Mutex,
    Condvar,

    mpsc,
};

pub struct Queue<T> {
    lock: Arc<Mutex<u16>>,
    condvar: Arc<Condvar>,
    blockingSender: mpsc::Sender<u16>,
    blockingRecever: mpsc::Receiver<u16>,

    queue: Vec<T>,
}

impl <T> Queue<T> {
    pub fn new() -> Queue<T> {
        let (blockingSender,blockingRecever) = mpsc::channel();

        return Queue {
            lock: Arc::new(Mutex::new(0_u16)),
            condvar: Arc::new(Condvar::new()),
            blockingSender,
            blockingRecever,

            queue: vec!(),
        };
    }
}
