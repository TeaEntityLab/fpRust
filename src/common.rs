/*!
In this module, there're many `trait`s/`struct`s and `fn`s defined,
for general purposes crossing over many modules of `fpRust`.
*/

use std::cmp::PartialEq;
use std::marker::PhantomData;
use std::time::{SystemTime, UNIX_EPOCH};

use std::sync::{Arc, Mutex};

// pub trait FnMutReceiveThreadSafe<X>: FnMut(Arc<X>) + Send + Sync + 'static {}
// pub trait FnMutReturnThreadSafe<X>: FnMut() -> X + Send + Sync + 'static {}

/**
Get a mut ref of a specific element of a Vec<T>.

# Arguments

* `v` - The mut ref of the `Vec<T>`
* `index` - The index of the element

# Remarks

This is a convenience function that gets the `mut ref` of an element of the `Vec`.

*/
pub fn get_mut<'a, T>(v: &'a mut Vec<T>, index: usize) -> Option<&'a mut T> {
    let mut i = 0;
    for elem in v {
        if index == i {
            return Some(elem);
        }
        i += 1;
    }

    None
}

/**
`Observable` memorizes all `Subscription` and send notifications.

# Arguments

* `X` - The generic type of broadcasted data
* `T` - The generic type implementing `trait` `Subscription`

# Remarks

This is an implementation of Observer Pattern of GoF.

*NOTE*: Inspired by and modified from https://github.com/eliovir/rust-examples/blob/master/design_pattern-observer.rs
*/
pub trait Observable<X, T: Subscription<X>> {
    fn add_observer(&mut self, observer: Arc<T>);
    fn delete_observer(&mut self, observer: Arc<T>);
    fn notify_observers(&mut self, x: Arc<X>);
}

/**
`Subscription` trait defines the interface of a broadcasting subscriber,
for general purposes crossing over many modules of fpRust.

# Arguments

* `X` - The generic type of broadcasted data

# Remarks

This is an implementation of Observer Pattern of GoF, and inspired by Rx Subscription.

*/
pub trait Subscription<X>: Send + Sync + 'static + PartialEq {
    fn on_next(&mut self, x: Arc<X>);
}

/**
`SubscriptionFunc` struct implements the interface of `Subscription`,
for general purposes crossing over many modules of fpRust.

# Arguments

* `T` - The generic type of broadcasted data

# Remarks

It's enough to use for general cases of `Subscription`.

*/
#[derive(Clone)]
pub struct SubscriptionFunc<T> {
    id: String,
    pub receiver: RawReceiver<T>,
}

impl<T: Send + Sync + 'static + Clone> SubscriptionFunc<T> {
    pub fn new(func: impl FnMut(Arc<T>) + Send + Sync + 'static) -> SubscriptionFunc<T> {
        let since_the_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        return SubscriptionFunc {
            id: format!("{:?}", since_the_epoch),
            receiver: RawReceiver::new(func),
        };
    }
}

impl<T: Send + Sync + 'static + Clone> PartialEq for SubscriptionFunc<T> {
    fn eq(&self, other: &SubscriptionFunc<T>) -> bool {
        self.id == other.id
    }
}

impl<T: Send + Sync + 'static + Clone> Subscription<T> for SubscriptionFunc<T> {
    fn on_next(&mut self, x: Arc<T>) {
        self.receiver.invoke(x);
    }
}

/**
`RawReceiver` struct implements an useful container of `FnMut`(`Arc<T>`)
, receiving an `Arc<T>` as its parameter.

# Arguments

* `T` - The generic type of received data.

# Remarks

It's the base of implementations of `SubscriptionFunc`.

*/
#[derive(Clone)]
pub struct RawReceiver<T> {
    func: Arc<Mutex<dyn FnMut(Arc<T>) + Send + Sync + 'static>>,
    _t: PhantomData<T>,
}

impl<T> RawReceiver<T> {
    pub fn new(func: impl FnMut(Arc<T>) + Send + Sync + 'static) -> RawReceiver<T> {
        return RawReceiver {
            func: Arc::new(Mutex::new(func)),
            _t: PhantomData,
        };
    }

    pub fn invoke(&self, x: Arc<T>) {
        let func = &mut *self.func.lock().unwrap();
        (func)(x);
    }
}

/**
`RawFunc` struct implements an useful container of `FnMut`,
which could be sent crossing `channel`s.

# Remarks

It's the base of sending `FnMut` objects crossing `channel`s.

*/
#[derive(Clone)]
pub struct RawFunc {
    func: Arc<Mutex<FnMut() + Send + Sync + 'static>>,
}

impl RawFunc {
    pub fn new<T>(func: T) -> RawFunc
    where
        T: FnMut() + Send + Sync + 'static,
    {
        return RawFunc {
            func: Arc::new(Mutex::new(func)),
        };
    }

    pub fn invoke(&self) {
        let func = &mut *self.func.lock().unwrap();
        (func)();
    }
}
