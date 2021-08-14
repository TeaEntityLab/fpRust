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
Run codes inside a `doM` block(`Haskell` `do notation`)
(this is a `sync` macro)

# Arguments

* `func` - The given `FnMut`, the execution code of `Cor`.

*/
#[macro_export]
macro_rules! do_m {
    ($this:expr) => {{
        let mut _do_m = cor_newmutex!($this, (), ());

        {
            let mut do_m = _do_m.lock().unwrap();
            do_m.set_async(false);
        }

        cor_start!(_do_m);
    }};
}

/**
Run codes inside a `doM` block(`Haskell` `do notation`)
(this is a `sync` macro)
*/
#[macro_export]
macro_rules! do_m_pattern {
    (
        let $p:pat = $e:expr ; $( $t:tt )*
    ) => (
        { let $p = $e ; do_m_pattern! { $( $t )* } }
    );
    (
        let $p:ident : $ty:ty = $e:expr ; $( $t:tt )*
    ) => (
        { let $p:$ty = $e ; do_m_pattern! { $( $t )* } }
    );
    (
        let mut $p:ident : $ty:ty = $e:expr ; $( $t:tt )*
    ) => (
        { let mut $p:$ty = $e ; do_m_pattern! { $( $t )* } }
    );
    (
        $p:ident = $e:expr ; $( $t:tt )*
    ) => (
        { $p = $e ; do_m_pattern! { $( $t )* } }
    );
    (
        exec $b:expr ; $( $t:tt )*
    ) => (
        { $b ; do_m_pattern! { $( $t )* } }
    );
    (
        ret $e:expr
    ) => (
        // vec!()
        $e
    );

    ($p:ident $ty:ty, $val:expr, yield_from $cor:expr; $($t:tt)*) => ({
        let $p: Arc<Mutex<Option<$ty>>> = Arc::new(Mutex::new(None::<$ty>));
        let _p = $p.clone();

        let mut _do_m = cor_newmutex!(
            move |this| {
                let mut p = _p.lock().unwrap();
                *p = cor_yield_from!(this, $cor, $val);
            },
            (),
            ()
        );

        {
            let mut do_m = _do_m.lock().unwrap();
            do_m.set_async(false);
        }

        cor_start!(_do_m);

        do_m_pattern! { $( $t )* }

        // let mut v = vec!(p);
        // let next = &mut do_m_pattern! { $( $t )* };
        // v.append(next);
        // v
    });
}

/**
Define a new `Cor` with type.
It will return a `Arc<Mutex<Cor>>`.

# Arguments

* `func` - The given `FnMut`, the execution code of `Cor`.
* `RETURN` - The type of returned data
* `RECEIVE` - The type of received data

*/
#[macro_export]
macro_rules! cor_newmutex {
    ($func:expr, $RETURN:ty, $RECEIVE:ty) => {
        <Cor<$RETURN, $RECEIVE>>::new_with_mutex($func)
    };
}

/**
Define a new `Cor` with type and start it immediately.
It will return a `Arc<Mutex<Cor>>`.

# Arguments

* `func` - The given `FnMut`, the execution code of `Cor`.
* `RETURN` - The type of returned data
* `RECEIVE` - The type of received data

*/
#[macro_export]
macro_rules! cor_newmutex_and_start {
    ($func:expr, $RETURN:ty, $RECEIVE:ty) => {{
        let new_one = <Cor<$RETURN, $RECEIVE>>::new_with_mutex($func);
        cor_start!(new_one);
        new_one
    }};
}

/**
Make `this` returns a given `Option<RETURN>` `given_to_outside` to its callee `Cor`,
and this method returns the `Option<RECEIVE>` value given from outside.

# Arguments

* `this` - The sender when sending `given_to_outside` to callee `Cor`.
* `given_to_outside` - The value sent by `this` and received by `target`.

# Remarks

This method is implemented according to some coroutine/generator implementations,
such as `Python`, `Lua`, `ECMASript`...etc.

*/
#[macro_export]
macro_rules! cor_yield {
    ($this:expr, $given_to_outside:expr) => {
        Cor::yield_ref($this, $given_to_outside)
    };
}

/**
Make `this` sends a given `Option<RECEIVETARGET>` to `target`,
and this method returns the `Option<RETURNTARGET>` response from `target`.

# Arguments

* `this` - The sender when sending `sent_to_inside` to `target`.
* `target` - The receiver of value `sent_to_inside` sent by `this`.
* `sent_to_inside` - The value sent by `this` and received by `target`.

# Remarks

This method is implemented according to some coroutine/generator implementations,
such as `Python`, `Lua`, `ECMASript`...etc.

*/
#[macro_export]
macro_rules! cor_yield_from {
    ($this:expr, $target:expr, $sent_to_inside:expr) => {
        Cor::yield_from($this.clone(), $target.clone(), $sent_to_inside)
    };
}

/**

Start `this` `Cor`.

# Arguments

* `this` - The target `Cor` to start.

*NOTE*: Beware the deadlock if it's sync(waiting for each other), except the entry point.

*/
#[macro_export]
macro_rules! cor_start {
    ($this:expr) => {
        Cor::start($this.clone())
    };
}

/**
`CorOp` defines a yield action between `Cor` objects.

# Arguments

* `RETURN` - The generic type of returned data
* `RECEIVE` - The generic type of received data

# Remarks

It's the base of implementations of `Cor`.
It contains the `Cor` calling `yield_from`() and the val sent together,
and it's necessary to the target `Cor` making the response by `yield_ref`()/`yield_none`().

*/
pub struct CorOp<RETURN: 'static, RECEIVE: 'static> {
    pub result_ch_sender: Arc<Mutex<Sender<Option<RETURN>>>>,
    pub val: Option<RECEIVE>,
}
impl<RETURN, RECEIVE> CorOp<RETURN, RECEIVE> {}

type CorEffect<RETURN, RECEIVE> =
    dyn FnMut(Arc<Mutex<Cor<RETURN, RECEIVE>>>) + Send + Sync + 'static;

/**
The `Cor` implements a *PythonicGenerator-like Coroutine*.

# Arguments

* `RETURN` - The generic type of returned data
* `RECEIVE` - The generic type of received data

# Remarks

It could be sync or async up to your usages,
and it could use `yield_from` to send a value to another `Cor` object and get the response,
and use `yield_ref`()/`yield_none`() to return my response to the callee of mine.

*NOTE*: Beware the deadlock if it's sync(waiting for each other), except the entry point.

*/
#[derive(Clone)]
pub struct Cor<RETURN: 'static, RECEIVE: 'static> {
    is_async: bool,

    started_alive: Arc<Mutex<(AtomicBool, AtomicBool)>>,

    op_ch_sender: Arc<Mutex<Sender<CorOp<RETURN, RECEIVE>>>>,
    op_ch_receiver: Arc<Mutex<Receiver<CorOp<RETURN, RECEIVE>>>>,
    effect: Arc<Mutex<CorEffect<RETURN, RECEIVE>>>,
}
impl<RETURN: Send + Sync + 'static, RECEIVE: Send + Sync + 'static> Cor<RETURN, RECEIVE> {
    /**
    Generate a new `Cor` with the given `FnMut` function for the execution of this `Cor`.

    # Arguments

    * `effect` - The given `FnMut`, the execution code of `Cor`.

    */
    pub fn new(
        effect: impl FnMut(Arc<Mutex<Cor<RETURN, RECEIVE>>>) + Send + Sync + 'static,
    ) -> Cor<RETURN, RECEIVE> {
        let (op_ch_sender, op_ch_receiver) = channel();
        Cor {
            is_async: true,
            started_alive: Arc::new(Mutex::new((AtomicBool::new(false), AtomicBool::new(false)))),

            op_ch_sender: Arc::new(Mutex::new(op_ch_sender)),
            op_ch_receiver: Arc::new(Mutex::new(op_ch_receiver)),
            effect: Arc::new(Mutex::new(effect)),
        }
    }

    /**
    Generate a new `Arc<Mutex<Cor<RETURN, RECEIVE>>>` with the given `FnMut` function for the execution of this `Cor`.

    # Arguments

    * `effect` - The given `FnMut`, the execution code of `Cor`.

    */
    pub fn new_with_mutex(
        effect: impl FnMut(Arc<Mutex<Cor<RETURN, RECEIVE>>>) + Send + Sync + 'static,
    ) -> Arc<Mutex<Cor<RETURN, RECEIVE>>> {
        Arc::new(Mutex::new(<Cor<RETURN, RECEIVE>>::new(effect)))
    }

    /**
    Make `this` sends a given `Option<RECEIVETARGET>` to `target`,
    and this method returns the `Option<RETURNTARGET>` response from `target`.

    # Arguments

    * `this` - The sender when sending `sent_to_inside` to `target`.
    * `target` - The receiver of value `sent_to_inside` sent by `this`.
    * `sent_to_inside` - The value sent by `this` and received by `target`.

    # Remarks

    This method is implemented according to some coroutine/generator implementations,
    such as `Python`, `Lua`, `ECMASript`...etc.

    */
    pub fn yield_from<RETURNTARGET: Send + Sync + 'static, RECEIVETARGET: Send + Sync + 'static>(
        this: Arc<Mutex<Cor<RETURN, RECEIVE>>>,
        target: Arc<Mutex<Cor<RETURNTARGET, RECEIVETARGET>>>,
        sent_to_inside: Option<RECEIVETARGET>,
    ) -> Option<RETURNTARGET> {
        // me MutexGuard lifetime block
        {
            let me = this.lock().unwrap();
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

            if let Ok(_x) = result {
                return _x;
            }
        }

        None
    }

    /**
    Make `this` returns a given `None::<RETURN>` to its callee `Cor`,
    and this method returns the `Option<RECEIVE>` value given from outside.

    # Arguments

    * `this` - The sender when sending `given_to_outside` to callee `Cor`.

    # Remarks

    This method is implemented according to some coroutine/generator implementations,
    such as `Python`, `Lua`, `ECMASript`...etc.

    */
    pub fn yield_none(this: Arc<Mutex<Cor<RETURN, RECEIVE>>>) -> Option<RECEIVE> {
        Cor::yield_ref(this, None)
    }

    /**
    Make `this` returns a given `Option<RETURN>` `given_to_outside` to its callee `Cor`,
    and this method returns the `Option<RECEIVE>` value given from outside.

    # Arguments

    * `this` - The sender when sending `given_to_outside` to callee `Cor`.
    * `given_to_outside` - The value sent by `this` and received by `target`.

    # Remarks

    This method is implemented according to some coroutine/generator implementations,
    such as `Python`, `Lua`, `ECMASript`...etc.

    */
    pub fn yield_ref(
        this: Arc<Mutex<Cor<RETURN, RECEIVE>>>,
        given_to_outside: Option<RETURN>,
    ) -> Option<RECEIVE> {
        let _op_ch_receiver;
        // me MutexGuard lifetime block
        {
            let me = this.lock().unwrap();
            if !me.is_alive() {
                return None;
            }
            _op_ch_receiver = me.op_ch_receiver.clone();
        }

        let op;
        {
            op = _op_ch_receiver.lock().unwrap().recv();
        }

        if let Ok(_x) = op {
            {
                let _result = _x.result_ch_sender.lock().unwrap().send(given_to_outside);
            }

            return _x.val;
        }

        None
    }

    /**

    Start `this` `Cor`.

    # Arguments

    * `this` - The target `Cor` to start.

    *NOTE*: Beware the deadlock if it's sync(waiting for each other), except the entry point.

    */
    pub fn start(this: Arc<Mutex<Cor<RETURN, RECEIVE>>>) {
        let is_async;

        {
            let me = this.lock().unwrap();
            is_async = me.is_async;

            let started_alive = me.started_alive.lock().unwrap();
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
                        _effect = this.lock().unwrap().effect.clone();
                    }

                    (_effect.lock().unwrap())(this.clone());
                }

                {
                    this.lock().unwrap().stop();
                }
            };
            if is_async {
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
    pub fn set_async(&mut self, is_async: bool) {
        self.is_async = is_async;
    }

    /**
    Did this `Cor` start?
    Return `true` when it did started (no matter it has stopped or not)

    */
    pub fn is_started(&self) -> bool {
        let started_alive = self.started_alive.lock().unwrap();
        let &(ref started, _) = &*started_alive;
        started.load(Ordering::SeqCst)
    }

    /**
    Is this `Cor` alive?
    Return `true` when it has started and not stopped yet.

    */
    pub fn is_alive(&self) -> bool {
        let started_alive = self.started_alive.lock().unwrap();
        let &(_, ref alive) = &*started_alive;
        alive.load(Ordering::SeqCst)
    }

    /**

    Stop `Cor`.
    This will make self.`is_alive`() returns `false`,
    and all `yield_from`() from this `Cor` as `target` will return `None::<RETURN>`.
    (Because it has stopped :P, that's reasonable)

    */
    pub fn stop(&mut self) {
        {
            let started_alive = self.started_alive.lock().unwrap();
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

    fn receive(
        &mut self,
        result_ch_sender: Arc<Mutex<Sender<Option<RETURN>>>>,
        given_as_request: Option<RECEIVE>,
    ) {
        // do_close_safe
        // if !self.is_alive() {
        let started_alive = self.started_alive.lock().unwrap();
        let &(_, ref alive) = &*started_alive;
        if !alive.load(Ordering::SeqCst) {
            return;
        }

        {
            let op_ch_sender = self.op_ch_sender.lock().unwrap();
            let _result = op_ch_sender.send(CorOp {
                // cor: cor,
                result_ch_sender,
                val: given_as_request,
            });
        }
    }

    /*
    fn do_close_safe(&mut self, mut effect: impl FnMut()) {
        if !self.is_alive() {
            return;
        }

        {
            let _started_alive = self.started_alive.lock().unwrap();

            (effect)();
        }
    }
    */
}

#[test]
fn test_cor_do_m() {
    let v = Arc::new(Mutex::new(String::from("")));

    let _v = v.clone();
    do_m!(move |this| {
        println!("test_cor_do_m started");

        let cor_inner1 = cor_newmutex_and_start!(
            |this| {
                let s = cor_yield!(this, Some(String::from("1")));
                println!("cor_inner1 {:?}", s);
            },
            String,
            i16
        );
        let cor_inner2 = cor_newmutex_and_start!(
            |this| {
                let s = cor_yield!(this, Some(String::from("2")));
                println!("cor_inner2 {:?}", s);
            },
            String,
            i16
        );
        let cor_inner3 = cor_newmutex_and_start!(
            |this| {
                let s = cor_yield!(this, Some(String::from("3")));
                println!("cor_inner3 {:?}", s);
            },
            String,
            i16
        );

        {
            (*_v.lock().unwrap()) = [
                cor_yield_from!(this, cor_inner1, Some(1)).unwrap(),
                cor_yield_from!(this, cor_inner2, Some(2)).unwrap(),
                cor_yield_from!(this, cor_inner3, Some(3)).unwrap(),
            ]
            .join("");
        }
    });

    let _v = v.clone();

    {
        assert_eq!("123", *_v.lock().unwrap());
    }
}

#[test]
fn test_cor_do_m_pattern() {
    let _r = do_m_pattern! {
        let mut _v4 = String::from("");
        _v4 = String::from("4");

        exec {
            println!("do_m_pattern _v4:{:?}", _v4)
        };

        _v1 String, Some(1), yield_from cor_newmutex_and_start!(
            |this| {
                let s = cor_yield!(this, Some(String::from("1")));
                println!("cor_inner1 {:?}", s);
            },
            String,
            i16
        );
        _v2 String, Some(2), yield_from cor_newmutex_and_start!(
            |this| {
                let s = cor_yield!(this, Some(String::from("2")));
                println!("cor_inner2 {:?}", s);
            },
            String,
            i16
        );
        _v3 String, Some(3), yield_from cor_newmutex_and_start!(
            |this| {
                let s = cor_yield!(this, Some(String::from("3")));
                println!("cor_inner3 {:?}", s);
            },
            String,
            i16
        );
        let _v1 = _v1.lock().unwrap();
        let _v2 = _v2.lock().unwrap();
        let _v3 = _v3.lock().unwrap();
        ret [
            _v1.clone().unwrap(),
            _v2.clone().unwrap(),
            _v3.clone().unwrap(),
            _v4,
        ].join("")
    };

    assert_eq!("1234", _r);
}

#[test]
fn test_cor_new() {
    use std::time;

    println!("test_cor_new");

    let _cor1 = cor_newmutex!(
        |this| {
            println!("cor1 started");

            let s = cor_yield!(this, Some(String::from("given_to_outside")));
            println!("cor1 {:?}", s);
        },
        String,
        i16
    );
    let cor1 = _cor1.clone();

    let _cor2 = cor_newmutex!(
        move |this| {
            println!("cor2 started");

            println!("cor2 yield_from before");

            let s = cor_yield_from!(this, cor1, Some(3));
            println!("cor2 {:?}", s);
        },
        i16,
        i16
    );

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
    cor_start!(_cor1);
    cor_start!(_cor2);

    thread::sleep(time::Duration::from_millis(1));
}
