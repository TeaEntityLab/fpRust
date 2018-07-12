use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
};
use std::thread;

pub struct CorOp<'a, X: 'a> {
    pub cor: &'a mut Cor<'a, X>,
    pub val: Option<X>,
}
impl<'a, X: Send + Sync + Clone> CorOp<'a, X> {}

#[derive(Clone)]
pub struct Cor<'a, X: 'a> {
    started_alive: Arc<Mutex<(AtomicBool, AtomicBool)>>,

    op_ch_sender: Arc<Mutex<Sender<CorOp<'a, X>>>>,
    op_ch_receiver: Arc<Mutex<Receiver<CorOp<'a, X>>>>,
    result_ch_sender: Arc<Mutex<Sender<Option<X>>>>,
    result_ch_receiver: Arc<Mutex<Receiver<Option<X>>>>,

    effect: Arc<Mutex<FnMut() + Send + Sync + 'static>>,
}
impl<'a, X: Send + Sync + Clone> Cor<'a, X> {
    pub fn new(effect: impl FnMut() + Send + Sync + 'static) -> Cor<'a, X> {
        let (op_ch_sender, op_ch_receiver) = channel();
        let (result_ch_sender, result_ch_receiver) = channel();
        return Cor {
            started_alive: Arc::new(Mutex::new((AtomicBool::new(false), AtomicBool::new(false)))),

            op_ch_sender: Arc::new(Mutex::new(op_ch_sender)),
            op_ch_receiver: Arc::new(Mutex::new(op_ch_receiver)),
            result_ch_sender: Arc::new(Mutex::new(result_ch_sender)),
            result_ch_receiver: Arc::new(Mutex::new(result_ch_receiver)),

            effect: Arc::new(Mutex::new(effect)),
        };
    }
    pub fn new_with_mutex(effect: impl FnMut() + Send + Sync + 'static) -> Arc<Mutex<Cor<'a, X>>> {
        return Arc::new(Mutex::new(<Cor<'a, X>>::new(effect)));
    }

    pub fn yield_from(
        &'a mut self,
        target: Arc<Mutex<Cor<'a, X>>>,
        given_to_outside: Option<X>,
    ) -> Option<X> {
        if !self.is_alive() {
            return None;
        }
        let _result_ch_receiver = self.result_ch_receiver.clone();

        // target MutexGuard lifetime block
        {
            target.lock().unwrap().receive(self, given_to_outside);

            let result = _result_ch_receiver.lock().unwrap().recv();

            match result.ok() {
                Some(_x) => {
                    return _x;
                }
                None => {}
            }
        }

        return None;
    }

    pub fn yield_none(&'a mut self) -> Option<X> {
        return Cor::yield_ref(self, None);
    }

    pub fn yield_ref(&'a mut self, given_to_outside: Option<X>) -> Option<X> {
        if !self.is_alive() {
            return None;
        }
        let _op_ch_receiver = self.op_ch_receiver.clone();

        let op;
        {
            let op_ch = &*_op_ch_receiver.lock().unwrap();
            op = op_ch.recv();
        }

        match op.ok() {
            Some(_x) => {
                {
                    let cor_other = _x.cor;
                    cor_other.offer(given_to_outside);
                }

                return _x.val;
            }
            None => {}
        }

        return None;
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

    pub fn start(&mut self) {
        {
            let _started_alive = self.started_alive.clone();
            let started_alive = _started_alive.lock().unwrap();
            let &(ref started, ref alive) = &*started_alive;
            if (!alive.load(Ordering::SeqCst)) || started.load(Ordering::SeqCst) {
                return;
            }
            started.store(true, Ordering::SeqCst);
            alive.store(true, Ordering::SeqCst);
        }

        {
            let _started_alive = self.started_alive.clone();

            let mut _effect = self.effect.clone();
            thread::spawn(move || {
                {
                    let effect = &mut *_effect.lock().unwrap();
                    (effect)();
                }

                {
                    let started_alive = _started_alive.lock().unwrap();
                    let &(_, ref alive) = &*started_alive;
                    alive.store(false, Ordering::SeqCst);
                }
            });
        }
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

    fn receive(&mut self, cor: &'a mut Cor<'a, X>, given_as_request: Option<X>) {
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
    let _cor1 = <Cor<String>>::new_with_mutex(|| {});
}
