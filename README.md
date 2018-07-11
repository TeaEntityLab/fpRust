# fpRust

[![tag](https://img.shields.io/github/tag/TeaEntityLab/fpRust.svg)](https://github.com/TeaEntityLab/fpRust)

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



~~* Pattern matching~~

~~* Fp functions~~ -> Currently only compose!() <- __macro__



~~* Java8Stream-like Collection~~

~~* PythonicGenerator-like Coroutine(yield/yieldFrom)~~


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

// fmap & map (sync)
let mut _s = Arc::new(SubscriptionFunc::new(move |x: Arc<u16>| {
    println!("Value: {:?}", x); // Value: 36
}));
let mut _s2 = _s.clone();
let monadioSync = MonadIO::new(of(1)).fmap(|x| MonadIO::new(move || x*4)).map(|x| x*3).map(|x| x*3);
monadioSync.subscribe(_s2);

// fmap & map (async)
let mut _h = HandlerThread::new();
let mut _h2 = HandlerThread::new();
let f4 = MonadIO::new_with_handlers(|| {
    String::from("ok")
    }, Some(_h.clone()), Some(_h2.clone()));

let h = Arc::make_mut(&mut _h);
let h2 = Arc::make_mut(&mut _h2);

let pair = Arc::new((Mutex::new(false), Condvar::new()));
let pair2 = pair.clone();

thread::sleep(time::Duration::from_millis(100));

h.start();
h2.start();

let s = Arc::new(SubscriptionFunc::new(move |x: Arc<String>| {
    println!("Value: {:?}", x); // Value: ok

    let &(ref lock, ref cvar) = &*pair2;
    let mut started = lock.lock().unwrap();
    *started = true;

    cvar.notify_one(); // Unlock here
}));
f4.subscribe(s);

thread::sleep(time::Duration::from_millis(100));

let &(ref lock, ref cvar) = &*pair;
let mut started = lock.lock().unwrap();
// Waiting for being unlcoked
while !*started {
    started = cvar.wait(started).unwrap();
}
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
