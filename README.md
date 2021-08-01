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
let mut _subscription = Arc::new(Mutex::new(SubscriptionFunc::new(move |x: Arc<u16>| {
    println!("monadio_sync {:?}", x); // monadio_sync 36
    assert_eq!(36, *Arc::make_mut(&mut x.clone()));
})));
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

thread::sleep(time::Duration::from_millis(100));

let subscription = Arc::new(Mutex::new(SubscriptionFunc::new(move |x: Arc<String>| {
    println!("monadio_async {:?}", x); // monadio_async ok

    latch2.countdown(); // Unlock here
})));
monadio_async.subscribe(subscription);
monadio_async.subscribe(Arc::new(Mutex::new(SubscriptionFunc::new(move |x: Arc<String>| {
    println!("monadio_async sub2 {:?}", x); // monadio_async sub2 ok
}))));
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
thread::sleep(time::Duration::from_millis(100));

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

let s = Arc::new(Mutex::new(SubscriptionFunc::new(move |x: Arc<String>| {
    println!("pub2-s1 I got {:?}", x);

    latch2.countdown();
})));
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

thread::sleep(time::Duration::from_millis(100));
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
