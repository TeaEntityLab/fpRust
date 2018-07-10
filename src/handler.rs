
use std::{
    thread,
};
use std::sync::{
    Arc,
    Mutex,
    atomic::{
        AtomicBool,
        Ordering,
    },
};

use common::RawFunc;
use sync as fpSync;
use sync::{
    Queue,
};

pub trait Handler : Send + Sync + 'static {
    fn is_started(&mut self) -> bool;
    fn is_alive(&mut self) -> bool;

    fn start(&mut self);
    fn stop(&mut self);

    fn post(&mut self, func : RawFunc);
}

#[derive(Clone)]
pub struct HandlerThread {
    started_alive: Arc<Mutex<(AtomicBool, AtomicBool)>>,

    inner: Arc<HandlerThreadInner>,

    handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

impl HandlerThread {
    pub fn new() -> Arc<HandlerThread> {
        return Arc::new(HandlerThread {
            started_alive: Arc::new(Mutex::new((AtomicBool::new(false), AtomicBool::new(false)))),
            inner: Arc::new(HandlerThreadInner::new()),

            handle: Arc::new(Mutex::new(None)),
        });
    }
}

impl Handler for HandlerThread {

    fn is_started(&mut self) -> bool {
        let _started_alive = self.started_alive.clone();
        let started_alive = _started_alive.lock().unwrap();
        let &(ref started, _) = &*started_alive;
        return started.load(Ordering::SeqCst);
    }

    fn is_alive(&mut self) -> bool {
        let _started_alive = self.started_alive.clone();
        let started_alive = _started_alive.lock().unwrap();
        let &(_, ref alive) = &*started_alive;
        return alive.load(Ordering::SeqCst);
    }

    fn start(&mut self) {

        {
            let _started_alive = self.started_alive.clone();
            let started_alive = _started_alive.lock().unwrap();
            let &(ref started, ref alive) = &*started_alive;

            if started.load(Ordering::SeqCst) {
                return;
            }
            started.store(true, Ordering::SeqCst);
            if alive.load(Ordering::SeqCst) {
                return;
            }
            alive.store(true, Ordering::SeqCst);
        }

        let mut _inner = self.inner.clone();

        self.handle = Arc::new(Mutex::new(Some(thread::spawn(move || {

            /*
            let inner : &mut HandlerThreadInner;
            let inner_temp = Arc::get_mut(&mut _inner);
            loop {
                match inner_temp {
                    Some(_x) => {
                        inner = _x;
                        // println!("True");
                        break;
                        },
                    None => {
                        println!("False");
                        continue;
                    }
                }
            }
            */
            Arc::make_mut(&mut _inner).start();
        }))));
    }

    fn stop(&mut self) {
        {
            let _started_alive = self.started_alive.clone();
            let started_alive = _started_alive.lock().unwrap();
            let &(ref started, ref alive) = &*started_alive;

            if !started.load(Ordering::SeqCst) {
                return;
            }
            if !alive.load(Ordering::SeqCst) {
                return;
            }
            alive.store(false, Ordering::SeqCst);
        }

        if !self.is_alive() {
            return;
        }
        Arc::make_mut(&mut self.inner).stop();

        let mut _handle = &mut self.handle;
        let option = Arc::get_mut(_handle);
        loop {
            match option {
                Some(m) => {
                    let mut handle = m.lock().unwrap();
                    handle
                        .take().expect("Called stop on non-running thread")
                        .join().expect("Could not join spawned thread");
                    break;
                },
                None => {
                    continue;
                }
            }
        }
    }

    fn post(&mut self, func: RawFunc) {
        Arc::make_mut(&mut self.inner).post(func);
    }
}

#[derive(Clone)]
struct HandlerThreadInner {
    // this: Option<Arc<HandlerThreadInner>>,

    started: Arc<AtomicBool>,
    alive: Arc<AtomicBool>,
    q: Arc<fpSync::BlockingQueue<RawFunc>>,
}

impl HandlerThreadInner {
    pub fn new() -> HandlerThreadInner {
        return HandlerThreadInner {
            started: Arc::new(AtomicBool::new(false)),
            alive: Arc::new(AtomicBool::new(false)),
            q: Arc::new(<fpSync::BlockingQueue<RawFunc>>::new()),
        };
    }

}

impl Handler for HandlerThreadInner {

    fn is_started(&mut self) -> bool {
        return self.started.load(Ordering::SeqCst);
    }

    fn is_alive(&mut self) -> bool {
        return self.alive.load(Ordering::SeqCst);
    }

    fn start(&mut self){
        self.alive.store(true, Ordering::SeqCst);
        let alive = self.alive.clone();

        if self.is_started() {
            return;
        }
        self.started.store(true, Ordering::SeqCst);

        let q = Arc::make_mut(&mut self.q);

        while alive.load(Ordering::SeqCst) {
            let v = q.take();

            v.invoke();
        }
    }

    fn stop(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
    }

    fn post(&mut self, func: RawFunc) {
        let q = Arc::make_mut(&mut self.q);

        q.put(func);
    }
}

#[test]
fn test_handler_new() {

    use std::{
        sync::{
            Condvar,
        },
        time,
    };

    let mut _h = HandlerThread::new();
    let h = Arc::make_mut(&mut _h);

    println!("is_alive {:?}", h.is_alive());
    println!("is_started {:?}", h.is_started());

    h.stop();
    h.stop();
    println!("is_alive {:?}", h.is_alive());
    println!("is_started {:?}", h.is_started());
    // let mut h1 = _h.clone();
    h.start();
    println!("is_alive {:?}", h.is_alive());
    println!("is_started {:?}", h.is_started());

    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair2 = pair.clone();

    // /*
    h.post(RawFunc::new(move ||{
        println!("Executed !");

        let pair3 = pair2.clone();

        let mut _h2 = HandlerThread::new();
        let mut _h2_inside = _h2.clone();

        let h2 = Arc::make_mut(&mut _h2);
        h2.start();

        h2.post(RawFunc::new(move ||{
            let &(ref lock, ref cvar) = &*pair3;
            let mut started = lock.lock().unwrap();
            *started = true;

            cvar.notify_one();

            Arc::make_mut(&mut _h2_inside).stop();
        }));

        }));
    println!("Test");

    thread::sleep(time::Duration::from_millis(100));

    println!("is_alive {:?}", h.is_alive());
    println!("is_started {:?}", h.is_started());

    h.stop();

    println!("is_alive {:?}", h.is_alive());
    println!("is_started {:?}", h.is_started());

    // /*

    let &(ref lock, ref cvar) = &*pair;
    let mut started = lock.lock().unwrap();
    while !*started {
        started = cvar.wait(started).unwrap();
    }
    // */
}
