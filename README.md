# fpRust

[![tag](https://img.shields.io/github/tag/TeaEntityLab/fpRust.svg)](https://github.com/TeaEntityLab/fpRust)
[![Crates.io](https://img.shields.io/crates/d/fp_rust.svg)](https://crates.io/crates/fp_rust)
[![Travis CI Build Status](https://api.travis-ci.org/TeaEntityLab/fpRust.svg?branch=master)](https://travis-ci.org/TeaEntityLab/fpRust)
[![docs](https://img.shields.io/badge/docs-online-5023dd.svg)](https://docs.rs/fp_rust/)

[![license](https://img.shields.io/github/license/TeaEntityLab/fpRust.svg?style=social&label=License)](https://github.com/TeaEntityLab/fpRust)
[![stars](https://img.shields.io/github/stars/TeaEntityLab/fpRust.svg?style=social&label=Stars)](https://github.com/TeaEntityLab/fpRust)
[![forks](https://img.shields.io/github/forks/TeaEntityLab/fpRust.svg?style=social&label=Fork)](https://github.com/TeaEntityLab/fpRust)

Monad, Functional Programming features for Rust

# Why

I love functional programming, Rx-style coding.

However it's hard to implement them in Rust, and there're few libraries to achieve parts of them.

Thus I implemented fpRust. I hope you would like it :)

# Features

* MonadIO, Rx-like (*`fp_rust::monadio::MonadIO`*)
  * map/fmap/subscribe
  * async/sync
  * Support *`Future`* (*`to_future()`*) with *`feature: for_futures`

* Publisher (*`fp_rust::publisher::Publisher`*)
  * Support *`Stream`* implementation(*`subscribe_as_stream()`*) with *`feature: for_futures`

* Fp functions (*`fp_rust::fp`*)
  * compose!(), pipe!()
  * map!(), reduce!(), filter!(), foldl!(), foldr!()
  * contains!(), reverse!()

* Async (*`fp_rust::sync`* & *`fp_rust::handler::HandlerThread`*)
  * simple BlockingQueue (inspired by *`Java BlockingQueue`*, implemented by built-in *`std::sync::mpsc::channel`*)
  * HandlerThread (inspired by *`Android Handler`*, implemented by built-in *`std::thread`*)
  * WillAsync (inspired by *`Java Future`*)
    * Support as a *`Future`* with *`feature: for_futures`
  * CountDownLatch (inspired by *`Java CountDownLatch`*, implemented by built-in *`std::sync::Mutex`*)
    * Support as a *`Future`* with *`feature: for_futures`

* Cor (*`fp_rust::cor::Cor`*)
  * PythonicGenerator-like Coroutine
  * yield/yieldFrom
  * async/sync

* Actor (*`fp_rust::actor::ActorAsync`*)
  * Pure simple *`Actor`* model(`receive`/`send`/`spawn`)
  * `Context` for keeping internal states
  * Able to communicate with Parent/Children Actors

* DoNotation (*`fp_rust::cor::Cor`*)
  * Haskell DoNotation-like, *macro*

~~* Pattern matching~~



# Usage

## MonadIO (RxObserver-like)

Example:
```rust

extern crate fp_rust;

use std::{
    thread,
    time,
    sync::{
        Arc,
        Mutex,
        Condvar,
    }
};

use fp_rust::handler::{
    Handler,
    HandlerThread,
};
use fp_rust::common::SubscriptionFunc;
use fp_rust::monadio::{
    MonadIO,
    of,
};
use fp_rust::sync::CountDownLatch;

// fmap & map (sync)
let mut _subscription = Arc::new(SubscriptionFunc::new(move |x: Arc<u16>| {
    println!("monadio_sync {:?}", x); // monadio_sync 36
    assert_eq!(36, *Arc::make_mut(&mut x.clone()));
}));
let subscription = _subscription.clone();
let monadio_sync = MonadIO::just(1)
    .fmap(|x| MonadIO::new(move || x * 4))
    .map(|x| x * 3)
    .map(|x| x * 3);
monadio_sync.subscribe(subscription);

// fmap & map (async)
let mut _handler_observe_on = HandlerThread::new_with_mutex();
let mut _handler_subscribe_on = HandlerThread::new_with_mutex();
let monadio_async = MonadIO::new_with_handlers(
    || {
        println!("In string");
        String::from("ok")
    },
    Some(_handler_observe_on.clone()),
    Some(_handler_subscribe_on.clone()),
);

let latch = CountDownLatch::new(1);
let latch2 = latch.clone();

thread::sleep(time::Duration::from_millis(1));

let subscription = Arc::new(SubscriptionFunc::new(move |x: Arc<String>| {
    println!("monadio_async {:?}", x); // monadio_async ok

    latch2.countdown(); // Unlock here
}));
monadio_async.subscribe(subscription);
monadio_async.subscribe(Arc::new(SubscriptionFunc::new(move |x: Arc<String>| {
    println!("monadio_async sub2 {:?}", x); // monadio_async sub2 ok
})));
{
    let mut handler_observe_on = _handler_observe_on.lock().unwrap();
    let mut handler_subscribe_on = _handler_subscribe_on.lock().unwrap();

    println!("hh2");
    handler_observe_on.start();
    handler_subscribe_on.start();
    println!("hh2 running");

    handler_observe_on.post(RawFunc::new(move || {}));
    handler_observe_on.post(RawFunc::new(move || {}));
    handler_observe_on.post(RawFunc::new(move || {}));
    handler_observe_on.post(RawFunc::new(move || {}));
    handler_observe_on.post(RawFunc::new(move || {}));
}
thread::sleep(time::Duration::from_millis(1));

// Waiting for being unlcoked
latch.clone().wait();
```

## Publisher (PubSub-like)

Example:
```rust

extern crate fp_rust;

use fp_rust::common::{SubscriptionFunc, RawFunc};
use fp_rust::handler::{Handler, HandlerThread};
use fp_rust::publisher::Publisher;
use std::sync::Arc;

use fp_rust::sync::CountDownLatch;

let mut pub1 = Publisher::new();
pub1.subscribe_fn(|x: Arc<u16>| {
    println!("pub1 {:?}", x);
    assert_eq!(9, *Arc::make_mut(&mut x.clone()));
});
pub1.publish(9);

let mut _h = HandlerThread::new_with_mutex();

let mut pub2 = Publisher::new_with_handlers(Some(_h.clone()));

let latch = CountDownLatch::new(1);
let latch2 = latch.clone();

let s = Arc::new(SubscriptionFunc::new(move |x: Arc<String>| {
    println!("pub2-s1 I got {:?}", x);

    latch2.countdown();
}));
pub2.subscribe(s.clone());
pub2.map(move |x: Arc<String>| {
    println!("pub2-s2 I got {:?}", x);
});

{
    let h = &mut _h.lock().unwrap();

    println!("hh2");
    h.start();
    println!("hh2 running");

    h.post(RawFunc::new(move || {}));
    h.post(RawFunc::new(move || {}));
    h.post(RawFunc::new(move || {}));
    h.post(RawFunc::new(move || {}));
    h.post(RawFunc::new(move || {}));
}

pub2.publish(String::from("OKOK"));
pub2.publish(String::from("OKOK2"));

pub2.unsubscribe(s.clone());

pub2.publish(String::from("OKOK3"));

latch.clone().wait();
```

## Cor (PythonicGenerator-like)

Example:
```rust

#[macro_use]
extern crate fp_rust;

use std::time;
use std::thread;

use fp_rust::cor::Cor;

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
```

## Do Notation (Haskell DoNotation-like)

Example:
```rust

#[macro_use]
extern crate fp_rust;

use std::time;
use std::thread;

use fp_rust::cor::Cor;


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
        ].join("");
    }
});

let _v = v.clone();

{
    assert_eq!("123", *_v.lock().unwrap());
}
```

## Fp Functions (Compose, Pipe, Map, Reduce, Filter)

Example:

```rust
#[macro_use]
extern crate fp_rust

use fp_rust::fp::{
  compose_two,
  map, reduce, filter,
};

let add = |x| x + 2;
let multiply = |x| x * 3;
let divide = |x| x / 2;

let result = (compose!(add, multiply, divide))(10);
assert_eq!(17, result);
println!("Composed FnOnce Result is {}", result);

let result = (pipe!(add, multiply, divide))(10);
assert_eq!(18, result);
println!("Piped FnOnce Result is {}", result);

let result = (compose!(reduce!(|a, b| a * b), filter!(|x| *x < 6), map!(|x| x * 2)))(vec![1, 2, 3, 4]);
assert_eq!(Some(8), result);
println!("test_map_reduce_filter Result is {:?}", result);
```

## Actor

### Actor common(send/receive/spawn/states)

Example:

```rust
use std::time::Duration;

use fp_rust::common::LinkedListAsync;

#[derive(Clone, Debug)]
enum Value {
    // Str(String),
    Int(i32),
    VecStr(Vec<String>),
    Spawn,
    Shutdown,
}

let result_i32 = LinkedListAsync::<i32>::new();
let result_i32_thread = result_i32.clone();
let result_string = LinkedListAsync::<Vec<String>>::new();
let result_string_thread = result_string.clone();
let mut root = ActorAsync::new(
    move |this: &mut ActorAsync<_, _>, msg: Value, context: &mut HashMap<String, Value>| {
        match msg {
            Value::Spawn => {
                println!("Actor Spawn");
                let result_i32_thread = result_i32_thread.clone();
                let spawned = this.spawn_with_handle(Box::new(
                    move |this: &mut ActorAsync<_, _>, msg: Value, _| {
                        match msg {
                            Value::Int(v) => {
                                println!("Actor Child Int");
                                result_i32_thread.push_back(v * 10);
                            }
                            Value::Shutdown => {
                                println!("Actor Child Shutdown");
                                this.stop();
                            }
                            _ => {}
                        };
                    },
                ));
                let list = context.get("children_ids").cloned();
                let mut list = match list {
                    Some(Value::VecStr(list)) => list,
                    _ => Vec::new(),
                };
                list.push(spawned.get_id());
                context.insert("children_ids".into(), Value::VecStr(list));
            }
            Value::Shutdown => {
                println!("Actor Shutdown");
                if let Some(Value::VecStr(ids)) = context.get("children_ids") {
                    result_string_thread.push_back(ids.clone());
                }

                for (id, handle) in this.children_handle_map.lock().unwrap().iter_mut() {
                    println!("Actor Shutdown id {:?}", id);
                    handle.send(Value::Shutdown);
                }
                this.stop();
            }
            Value::Int(v) => {
                println!("Actor Int");
                if let Some(Value::VecStr(ids)) = context.get("children_ids") {
                    for id in ids {
                        println!("Actor Int id {:?}", id);
                        if let Some(mut handle) = this.get_handle_child(id) {
                            handle.send(Value::Int(v));
                        }
                    }
                }
            }
            _ => {}
        }
    },
);

let mut root_handle = root.get_handle();
root.start();

// One child
root_handle.send(Value::Spawn);
root_handle.send(Value::Int(10));
// Two children
root_handle.send(Value::Spawn);
root_handle.send(Value::Int(20));
// Three children
root_handle.send(Value::Spawn);
root_handle.send(Value::Int(30));

// Send Shutdown
root_handle.send(Value::Shutdown);

thread::sleep(Duration::from_millis(1));
// 3 children Actors
assert_eq!(3, result_string.pop_front().unwrap().len());

let mut v = Vec::<Option<i32>>::new();
for _ in 1..7 {
    let i = result_i32.pop_front();
    println!("Actor {:?}", i);
    v.push(i);
}
v.sort();
assert_eq!(
    [
        Some(100),
        Some(200),
        Some(200),
        Some(300),
        Some(300),
        Some(300)
    ],
    v.as_slice()
)
```

### Actor Ask (inspired by Akka/Erlang)

Example:

```rust
use std::time::Duration;

use fp_rust::common::LinkedListAsync;

#[derive(Clone, Debug)]
enum Value {
    AskIntByLinkedListAsync((i32, LinkedListAsync<i32>)),
    AskIntByBlockingQueue((i32, BlockingQueue<i32>)),
}

let mut root = ActorAsync::new(
    move |_: &mut ActorAsync<_, _>, msg: Value, _: &mut HashMap<String, Value>| match msg {
        Value::AskIntByLinkedListAsync(v) => {
            println!("Actor AskIntByLinkedListAsync");
            v.1.push_back(v.0 * 10);
        }
        Value::AskIntByBlockingQueue(mut v) => {
            println!("Actor AskIntByBlockingQueue");

            // NOTE If negative, hanging for testing timeout
            if v.0 < 0 {
                return;
            }

            // NOTE General Cases
            v.1.offer(v.0 * 10);
        } // _ => {}
    },
);

let mut root_handle = root.get_handle();
root.start();

// LinkedListAsync<i32>
let result_i32 = LinkedListAsync::<i32>::new();
root_handle.send(Value::AskIntByLinkedListAsync((1, result_i32.clone())));
root_handle.send(Value::AskIntByLinkedListAsync((2, result_i32.clone())));
root_handle.send(Value::AskIntByLinkedListAsync((3, result_i32.clone())));
thread::sleep(Duration::from_millis(1));
let i = result_i32.pop_front();
assert_eq!(Some(10), i);
let i = result_i32.pop_front();
assert_eq!(Some(20), i);
let i = result_i32.pop_front();
assert_eq!(Some(30), i);

// BlockingQueue<i32>
let mut result_i32 = BlockingQueue::<i32>::new();
result_i32.timeout = Some(Duration::from_millis(1));
root_handle.send(Value::AskIntByBlockingQueue((4, result_i32.clone())));
root_handle.send(Value::AskIntByBlockingQueue((5, result_i32.clone())));
root_handle.send(Value::AskIntByBlockingQueue((6, result_i32.clone())));
thread::sleep(Duration::from_millis(1));
let i = result_i32.take();
assert_eq!(Some(40), i);
let i = result_i32.take();
assert_eq!(Some(50), i);
let i = result_i32.take();
assert_eq!(Some(60), i);

// Timeout case:
root_handle.send(Value::AskIntByBlockingQueue((-1, result_i32.clone())));
let i = result_i32.take();
assert_eq!(None, i);
```
