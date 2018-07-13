# fpRust

[![tag](https://img.shields.io/github/tag/TeaEntityLab/fpRust.svg)](https://github.com/TeaEntityLab/fpRust)
[![Crates.io](https://img.shields.io/crates/d/fp_rust.svg)](https://crates.io/crates/fp_rust)
[![docs](https://img.shields.io/badge/docs-online-5023dd.svg)](https://docs.rs/fp_rust/)

[![license](https://img.shields.io/github/license/TeaEntityLab/fpRust.svg?style=social&label=License)](https://github.com/TeaEntityLab/fpRust)
[![stars](https://img.shields.io/github/stars/TeaEntityLab/fpRust.svg?style=social&label=Stars)](https://github.com/TeaEntityLab/fpRust)
[![forks](https://img.shields.io/github/forks/TeaEntityLab/fpRust.svg?style=social&label=Fork)](https://github.com/TeaEntityLab/fpRust)

Monad, Functional Programming features for Rust

# Why

I love functional programing, Rx-style coding.

However it's hard to implement them in Rust, and there're few libraries to achieve parts of them.

Thus I implemented fpRust. I hope you would like it :)

# Features

* Optional/Maybe (a wrapper to built-in Option<T>, to make it more like a monad version *`Maybe`*)

* Monad, Rx-like

* Publisher

* Fp functions
  * Currently only compose!() <- __macro__

* Async
  * simple BlockingQueue (inspired by *`Java BlockingQueue`*, implemented by built-in *`std::sync::mpsc::channel`*)
  * HandlerThread (inspired by *`Android Handler`*, implemented by built-in *`std::thread`*)

* Cor
  * PythonicGenerator-like Coroutine
  * yield/yieldFrom
  * async/sync

~~* Pattern matching~~



# Usage

## Optional (IsPresent/IsNil, Or, Let)

```rust
extern crate fp_rust;

use fp_rust::maybe::Maybe;

// fmap & map (sync)
Maybe::val(true).fmap(|x| {return Maybe::val(!x.unwrap())}).unwrap(); // false
Maybe::val(false).fmap(|x| {return Maybe::val(!x.unwrap())}).unwrap(); // true

Maybe::val(true).map(|x| {return Some(!x.unwrap())}).unwrap(); // false
Maybe::val(false).map(|x| {return Some(!x.unwrap())}).unwrap(); // true

// fantasy-land: Apply ap()
Maybe::val(1).ap(
   Maybe::val(|x: Option<i16>| {
       if x.unwrap() > 0 {
           return Some(true)
       } else {
           return Some(false)
       }
   })
).unwrap(); // true

// or
Maybe::just(None::<bool>).or(false); // false
Maybe::val(true).or(false); // true

// unwrap
Maybe::val(true).unwrap(); //true

use std::panic;
let none_unwrap = panic::catch_unwind(|| {
    Maybe::just(None::<bool>).unwrap();
});
none_unwrap.is_err(); //true

// Get raw Option<T>
let v = match Maybe::val(true).option() {
    None => false,
    Some(_x) => true,
}; // true
let v = match Maybe::just(None::<bool>).option() {
    None => false,
    Some(_x) => true,
}; // false
```

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

thread::sleep(time::Duration::from_millis(100));

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

let s = Arc::new(SubscriptionFunc::new(move |x: Arc<String>| {
    println!("pub2-s1 I got {:?}", x);

    latch2.countdown();
}));
pub2.subscribe(s);
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

latch.clone().wait();
```

## Cor (PythonicGenerator-like)

Example:
```rust

extern crate fp_rust;

use std::time;
use std::thread;

use fp_rust::cor::Cor;

println!("test_cor_new");

let _cor1 = <Cor<String>>::new_with_mutex(|cor1| {
    println!("cor1 started");

    let s = Cor::yield_ref(cor1.clone(), Some(String::from("given_to_outside")));
    println!("cor1 {:?}", s);
});
let cor1 = _cor1.clone();

let _cor2 = <Cor<String>>::new_with_mutex(move |cor2| {
    println!("cor2 started");

    println!("cor2 yield_from before");

    let s = Cor::yield_from(cor2.clone(), cor1.clone(), Some(String::from("3")));
    println!("cor2 {:?}", s);
});
let cor2 = _cor2.clone();

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
```

## Compose

Example:

```rust
#[macro_use]
extern crate fp_rust

use fp_rust::fp::compose_two;

let add = |x| x + 2;
let multiply = |x| x * 2;
let divide = |x| x / 2;
let intermediate = compose!(add, multiply, divide);

println!("Value: {}", intermediate(10)); // Value: 12
```
