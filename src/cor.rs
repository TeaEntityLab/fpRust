use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
};
use std::thread;

pub struct CorOp<X: 'static> {
    pub cor: Arc<Mutex<Cor<X>>>,
    pub val: Option<X>,
}
impl<X: Send + Sync + Clone> CorOp<X> {}

#[derive(Clone)]
pub struct Cor<X: 'static> {
    async: bool,

    started_alive: Arc<Mutex<(AtomicBool, AtomicBool)>>,

    op_ch_sender: Arc<Mutex<Sender<CorOp<X>>>>,
    op_ch_receiver: Arc<Mutex<Receiver<CorOp<X>>>>,
    result_ch_sender: Arc<Mutex<Sender<Option<X>>>>,
    result_ch_receiver: Arc<Mutex<Receiver<Option<X>>>>,

    effect: Arc<Mutex<FnMut(Arc<Mutex<Cor<X>>>) + Send + Sync + 'static>>,
}
impl<X: Send + Sync + Clone + 'static> Cor<X> {
    pub fn new(effect: impl FnMut(Arc<Mutex<Cor<X>>>) + Send + Sync + 'static) -> Cor<X> {
        let (op_ch_sender, op_ch_receiver) = channel();
        let (result_ch_sender, result_ch_receiver) = channel();
        Cor {
            async: true,
            started_alive: Arc::new(Mutex::new((AtomicBool::new(false), AtomicBool::new(false)))),

            op_ch_sender: Arc::new(Mutex::new(op_ch_sender)),
            op_ch_receiver: Arc::new(Mutex::new(op_ch_receiver)),
            result_ch_sender: Arc::new(Mutex::new(result_ch_sender)),
            result_ch_receiver: Arc::new(Mutex::new(result_ch_receiver)),

            effect: Arc::new(Mutex::new(effect)),
        }
    }
    pub fn new_with_mutex(
        effect: impl FnMut(Arc<Mutex<Cor<X>>>) + Send + Sync + 'static,
    ) -> Arc<Mutex<Cor<X>>> {
        Arc::new(Mutex::new(<Cor<X>>::new(effect)))
    }

    pub fn yield_from(
        this: Arc<Mutex<Cor<X>>>,
        target: Arc<Mutex<Cor<X>>>,
        given_to_outside: Option<X>,
    ) -> Option<X> {
        let _result_ch_receiver;

        // me MutexGuard lifetime block
        {
            let _me = this.clone();
            let mut me = _me.lock().unwrap();
            if !me.is_alive() {
                return None;
            }
            _result_ch_receiver = me.result_ch_receiver.clone();
        }

        // target MutexGuard lifetime block
        {
            {
                target
                    .lock()
                    .unwrap()
                    .receive(this.clone(), given_to_outside);
            }

            let result;
            {
                let result_ch_receiver = _result_ch_receiver.lock().unwrap();
                result = result_ch_receiver.recv();
            }

            match result.ok() {
                Some(_x) => {
                    return _x;
                }
                None => {}
            }
        }

        return None;
    }

    pub fn yield_none(this: Arc<Mutex<Cor<X>>>) -> Option<X> {
        return Cor::yield_ref(this, None);
    }

    pub fn yield_ref(this: Arc<Mutex<Cor<X>>>, given_to_outside: Option<X>) -> Option<X> {
        let _op_ch_receiver;
        // me MutexGuard lifetime block
        {
            let _me = this.clone();
            let mut me = _me.lock().unwrap();
            if !me.is_alive() {
                return None;
            }
            _op_ch_receiver = me.op_ch_receiver.clone();
        }

        let op;
        {
            let op_ch = &*_op_ch_receiver.lock().unwrap();
            op = op_ch.recv();
        }

        match op.ok() {
            Some(_x) => {
                {
                    let mut cor_other = _x.cor.lock().unwrap();
                    cor_other.offer(given_to_outside);
                }

                return _x.val;
            }
            None => {}
        }

        return None;
    }

    pub fn start(this: Arc<Mutex<Cor<X>>>) {
        let async;

        {
            let _started_alive;

            {
                let _me = this.clone();
                let me = _me.lock().unwrap();
                async = me.async;
                _started_alive = me.started_alive.clone();
            }

            let started_alive = _started_alive.lock().unwrap();
            let &(ref started, ref alive) = &*started_alive;
            if started.load(Ordering::SeqCst) {
                return;
            }
            started.store(true, Ordering::SeqCst);
            alive.store(true, Ordering::SeqCst);
        }

        {
            let do_things = move || {
                {
                    let mut _effect;
                    {
                        let _me = this.clone();
                        let me = _me.lock().unwrap();
                        _effect = me.effect.clone();
                    }

                    let effect = &mut *_effect.lock().unwrap();
                    (effect)(this.clone());
                }

                {
                    let _me = this.clone();
                    let mut me = _me.lock().unwrap();
                    me.stop();
                }
            };
            if async {
                thread::spawn(do_things);
            } else {
                do_things();
            }
        }
    }

    pub fn set_async(&mut self, async: bool) {
        self.async = async;
    }

    pub fn is_started(&mut self) -> bool {
        let _started_alive = self.started_alive.clone();
        let started_alive = _started_alive.lock().unwrap();
        let &(ref started, _) = &*started_alive;
        return started.load(Ordering::SeqCst);
    }

    pub fn is_alive(&mut self) -> bool {
        let _started_alive = self.started_alive.clone();
        let started_alive = _started_alive.lock().unwrap();
        let &(_, ref alive) = &*started_alive;
        return alive.load(Ordering::SeqCst);
    }

    pub fn stop(&mut self) {
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

            {
                drop(self.op_ch_sender.lock().unwrap());
                drop(self.result_ch_sender.lock().unwrap());
            }
        }
    }

    fn receive(&mut self, cor: Arc<Mutex<Cor<X>>>, given_as_request: Option<X>) {
        let _op_ch_sender = self.op_ch_sender.clone();
        let _given_as_request = Box::new(given_as_request);

        // do_close_safe
        if !self.is_alive() {
            return;
        }

        {
            let __started_alive = self.started_alive.clone();
            let _started_alive = __started_alive.lock().unwrap();

            // do: (effect)();
            let given_as_request = _given_as_request.clone();
            let _result = _op_ch_sender.lock().unwrap().send(CorOp {
                cor: cor,
                val: *given_as_request,
            });
        }
    }

    fn offer(&mut self, received_as_response: Option<X>) {
        let _result_ch_sender = self.result_ch_sender.clone();
        let _received_as_response = Box::new(received_as_response);
        self.do_close_safe(move || {
            let result_ch_sender = _result_ch_sender.clone();
            let received_as_response = _received_as_response.clone();

            let _result = result_ch_sender.lock().unwrap().send(*received_as_response);
        });
    }

    fn do_close_safe(&mut self, mut effect: impl FnMut()) {
        if !self.is_alive() {
            return;
        }

        {
            let __started_alive = self.started_alive.clone();
            let _started_alive = __started_alive.lock().unwrap();

            (effect)();
        }
    }
}
#[test]
fn test_cor_new() {
    use std::time;

    println!("test_cor_new");

    let _cor1 = <Cor<String>>::new_with_mutex(|cor1| {
        println!("cor1 started");

        let _cor2 = <Cor<String>>::new_with_mutex(|cor2| {
            println!("cor2 started");

            let s = Cor::yield_ref(cor2, Some(String::from("given_to_outside")));
            println!("cor2 {:?}", s);
        });
        Cor::start(_cor2.clone());

        let s = Cor::yield_from(cor1.clone(), _cor2.clone(), Some(String::from("3")));
        println!("cor1 {:?}", s);
    });
    Cor::start(_cor1);

    thread::sleep(time::Duration::from_millis(100));
}