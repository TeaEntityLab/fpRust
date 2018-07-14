/*!
In this module there're implementations & tests of `Cor`.
*/
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
};
use std::thread;

/**
`CorOp` defines a yield action between `Cor` objects.

# Arguments

* `X` - The generic type of yielded data

# Remarks

It's the base of implementations of `Cor`.
It contains the `Cor` calling `yield_from`() and the val sent together,
and it's necessary to the target `Cor` making the response by `yield_ref`()/`yield_none`().

*/
pub struct CorOp<X: 'static> {
    pub result_ch_sender: Arc<Mutex<Sender<Option<X>>>>,
    pub val: Option<X>,
}
impl<X> CorOp<X> {}

/**
The `Cor` implements a *PythonicGenerator-like Coroutine*.

# Arguments

* `X` - The generic type of yielded/yielding data

# Remarks

It could be sync or async up to your usages,
and it could use `yield_from` to send a value to another `Cor` object and get the response,
and use `yield_ref`()/`yield_none`() to return my response to the callee of mine.

*NOTE*: Beware the deadlock if it's sync(waiting for each other), except the entry point.

*/
#[derive(Clone)]
pub struct Cor<X: 'static> {
    async: bool,

    started_alive: Arc<Mutex<(AtomicBool, AtomicBool)>>,

    op_ch_sender: Arc<Mutex<Sender<CorOp<X>>>>,
    op_ch_receiver: Arc<Mutex<Receiver<CorOp<X>>>>,
    effect: Arc<Mutex<FnMut(Arc<Mutex<Cor<X>>>) + Send + Sync + 'static>>,
}
impl<X: Send + Sync + Clone + 'static> Cor<X> {
    /**
    Generate a new `Cor` with the given `FnMut` function for the execution of this `Cor`.

    # Arguments

    * `effect` - The given `FnMut`, the execution code of `Cor`.

    */
    pub fn new(effect: impl FnMut(Arc<Mutex<Cor<X>>>) + Send + Sync + 'static) -> Cor<X> {
        let (op_ch_sender, op_ch_receiver) = channel();
        Cor {
            async: true,
            started_alive: Arc::new(Mutex::new((AtomicBool::new(false), AtomicBool::new(false)))),

            op_ch_sender: Arc::new(Mutex::new(op_ch_sender)),
            op_ch_receiver: Arc::new(Mutex::new(op_ch_receiver)),
            effect: Arc::new(Mutex::new(effect)),
        }
    }

    /**
    Generate a new `Arc<Mutex<Cor<X>>>` with the given `FnMut` function for the execution of this `Cor`.

    # Arguments

    * `effect` - The given `FnMut`, the execution code of `Cor`.

    */
    pub fn new_with_mutex(
        effect: impl FnMut(Arc<Mutex<Cor<X>>>) + Send + Sync + 'static,
    ) -> Arc<Mutex<Cor<X>>> {
        Arc::new(Mutex::new(<Cor<X>>::new(effect)))
    }

    /**
    Make `this` sends a given `Option<X>` to `target`,
    and this method returns the response from `target`.

    # Arguments

    * `this` - The sender when sending `sent_to_inside` to `target`.
    * `target` - The receiver of value `sent_to_inside` sent by `this`.
    * `sent_to_inside` - The value sent by `this` and received by `target`.

    # Remarks

    This method is implemented according to some coroutine/generator implementations,
    such as `Python`, `Lua`, `ECMASript`...etc.

    */
    pub fn yield_from<Y: Send + Sync + Clone + 'static>(
        this: Arc<Mutex<Cor<X>>>,
        target: Arc<Mutex<Cor<Y>>>,
        sent_to_inside: Option<Y>,
    ) -> Option<Y> {
        // me MutexGuard lifetime block
        {
            let _me = this.clone();
            let mut me = _me.lock().unwrap();
            if !me.is_alive() {
                return None;
            }
        }

        // target MutexGuard lifetime block
        {
            let (result_ch_sender, result_ch_receiver) = channel();
            let _result_ch_sender = Arc::new(Mutex::new(result_ch_sender));
            let _result_ch_receiver = Arc::new(Mutex::new(result_ch_receiver));

            {
                target
                    .lock()
                    .unwrap()
                    .receive(_result_ch_sender.clone(), sent_to_inside);
            }

            let result;
            {
                let result_ch_receiver = _result_ch_receiver.lock().unwrap();
                result = result_ch_receiver.recv();
            }
            {
                drop(_result_ch_sender.lock().unwrap());
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

    /**
    Make `this` returns a given `None::<X>` to its callee `Cor`,
    and this method returns the value given from outside.

    # Arguments

    * `this` - The sender when sending `given_to_outside` to callee `Cor`.

    # Remarks

    This method is implemented according to some coroutine/generator implementations,
    such as `Python`, `Lua`, `ECMASript`...etc.

    */
    pub fn yield_none(this: Arc<Mutex<Cor<X>>>) -> Option<X> {
        return Cor::yield_ref(this, None);
    }

    /**
    Make `this` returns a given `Option<X>` `given_to_outside` to its callee `Cor`,
    and this method returns the value given from outside.

    # Arguments

    * `this` - The sender when sending `given_to_outside` to callee `Cor`.
    * `given_to_outside` - The value sent by `this` and received by `target`.

    # Remarks

    This method is implemented according to some coroutine/generator implementations,
    such as `Python`, `Lua`, `ECMASript`...etc.

    */
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
                    let _result = _x.result_ch_sender.lock().unwrap().send(given_to_outside);
                }

                return _x.val;
            }
            None => {}
        }

        return None;
    }

    /**

    Start `this` `Cor`.

    # Arguments

    * `this` - The target `Cor` to start.

    *NOTE*: Beware the deadlock if it's sync(waiting for each other), except the entry point.

    */
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

    /**
    Setup async or not.
    Default `async`: `true`

    # Arguments

    * `async` - async when `true`, otherwise `sync`.

    *NOTE*: Beware the deadlock if it's sync(waiting for each other), except the entry point.

    */
    pub fn set_async(&mut self, async: bool) {
        self.async = async;
    }

    /**
    Did this `Cor` start?
    Return `true` when it did started (no matter it has stopped or not)

    */
    pub fn is_started(&mut self) -> bool {
        let _started_alive = self.started_alive.clone();
        let started_alive = _started_alive.lock().unwrap();
        let &(ref started, _) = &*started_alive;
        return started.load(Ordering::SeqCst);
    }

    /**
    Is this `Cor` alive?
    Return `true` when it has started and not stopped yet.

    */
    pub fn is_alive(&mut self) -> bool {
        let _started_alive = self.started_alive.clone();
        let started_alive = _started_alive.lock().unwrap();
        let &(_, ref alive) = &*started_alive;
        return alive.load(Ordering::SeqCst);
    }

    /**

    Stop `Cor`.
    This will make self.`is_alive`() returns `false`,
    and all `yield_from`() from this `Cor` as `target` will return `None::<X>`.
    (Because it has stopped :P, that's reasonable)

    */
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
            }
        }
    }

    // fn receive(&mut self, cor: Arc<Mutex<Cor<X>>>, given_as_request: Option<X>) {
    fn receive(
        &mut self,
        result_ch_sender: Arc<Mutex<Sender<Option<X>>>>,
        given_as_request: Option<X>,
    ) {
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
                // cor: cor,
                result_ch_sender: result_ch_sender,
                val: *given_as_request,
            });
        }
    }

    /*
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
    */
}
#[test]
fn test_cor_new() {
    use std::time;

    println!("test_cor_new");

    let _cor1 = <Cor<String>>::new_with_mutex(|this| {
        println!("cor1 started");

        let s = Cor::yield_ref(this.clone(), Some(String::from("given_to_outside")));
        println!("cor1 {:?}", s);
    });
    let cor1 = _cor1.clone();

    let _cor2 = <Cor<i16>>::new_with_mutex(move |this| {
        println!("cor2 started");

        println!("cor2 yield_from before");

        let s = Cor::yield_from(this.clone(), cor1.clone(), Some(String::from("3")));
        println!("cor2 {:?}", s);
    });

    {
        let cor1 = _cor1.clone();
        cor1.lock().unwrap().set_async(true); // NOTE Cor default async
                                              // NOTE cor1 should keep async to avoid deadlock waiting.(waiting for each other)
    }
    {
        let cor2 = _cor2.clone();
        cor2.lock().unwrap().set_async(false);
        // NOTE cor2 is the entry point, so it could be sync without any deadlock.
    }
    Cor::start(_cor1.clone());
    Cor::start(_cor2.clone());

    thread::sleep(time::Duration::from_millis(100));
}
