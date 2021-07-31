/*!
In this module, there're many `trait`s/`struct`s and `fn`s defined,
for general purposes crossing over many modules of `fpRust`.
*/

use std::cmp::PartialEq;
use std::marker::PhantomData;

use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use std::sync::{Arc, Mutex};

#[cfg(feature = "for_futures")]
use std::collections::VecDeque;
#[cfg(feature = "for_futures")]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "for_futures")]
use futures::stream::Stream;

#[cfg(feature = "for_futures")]
use super::sync::{BlockingQueue, Queue};

// pub trait FnMutReceiveThreadSafe<X>: FnMut(Arc<X>) + Send + Sync + 'static {}
// pub trait FnMutReturnThreadSafe<X>: FnMut() -> X + Send + Sync + 'static {}

/**

Insert `key-value` pairs into the given `map`

# Arguments

* `target` - The target `HashMap` to insert.
* `key, value` - `key-value` pairs.

*/
#[macro_export]
macro_rules! map_insert {
    ($target:ident, {
        $($key:ident : $value:expr,)*
    }) => {
        $(
            $target.insert(stringify!($key), $value);
        )*
    };
    ($target:ident, [
        $($key:ident : $value:expr,)*
    ]) => {
        $(
            $target.insert(stringify!($key), $value);
        )*
    };
    ($target:ident, {
        $($key:expr => $value:expr,)*
    }) => {
        $(
            $target.insert($key, $value);
        )*
    };
    ($target:ident, [
        $($key:expr => $value:expr,)*
    ]) => {
        $(
            $target.insert($key, $value);
        )*
    };
    ($target:ident, [
        $($key:expr, $value:expr,)*
    ]) => {
        $(
            $target.insert($key, $value);
        )*
    };
}

/**
Get a mut ref of a specific element of a Vec<T>.

# Arguments

* `v` - The mut ref of the `Vec<T>`
* `index` - The index of the element

# Remarks

This is a convenience function that gets the `mut ref` of an element of the `Vec`.

*/
pub fn get_mut<'a, T>(v: &'a mut Vec<T>, index: usize) -> Option<&'a mut T> {
    for (i, elem) in v.into_iter().enumerate() {
        if index == i {
            return Some(elem);
        }
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
    /**
    Add a `Subscription`.

    # Arguments

    * `observer` - The given `Subscription`.

    */
    fn add_observer(&mut self, observer: Arc<Mutex<T>>);

    /**
    Remove the observer.

    # Arguments

    * `observer` - The given `Subscription`.

    */
    fn delete_observer(&mut self, observer: Arc<Mutex<T>>);

    /**
    Notify all `Subscription` subscribers with a given value `Arc<X>`.

    # Arguments

    * `x` - The given `Arc<X>` value for broadcasting.

    */
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
pub trait Subscription<X>: Send + Sync + 'static + UniqueId<String> {
    /**
    The callback when `Subscription` received the broadcasted value.

    # Arguments

    * `func` - The given `FnMut`.

    */
    fn on_next(&mut self, x: Arc<X>);
}

/**
`UniqueId` trait defines the interface of an object with an unique id,
for general purposes crossing over many modules of fpRust.

# Remarks

This is inspired by Java/Swift Hashable.

*/
pub trait UniqueId<T> {
    /**
    The callback when `Subscription` received the broadcasted value.

    # Arguments

    * `func` - The given `FnMut`.

    */
    fn get_id(&self) -> T;
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

    #[cfg(feature = "for_futures")]
    streams: Option<Arc<Mutex<VecDeque<BlockingQueue<Option<Arc<T>>>>>>>,
    #[cfg(feature = "for_futures")]
    alive: Option<Arc<Mutex<AtomicBool>>>,
}

#[cfg(feature = "for_futures")]
impl<T> SubscriptionFunc<T> {
    pub fn close(&self) {
        match &self.alive {
            Some(alive) => alive.lock().unwrap().store(false, Ordering::SeqCst),
            None => {}
        }
    }
}

impl<T: Send + Sync + 'static> SubscriptionFunc<T> {
    pub fn new(func: impl FnMut(Arc<T>) + Send + Sync + 'static) -> SubscriptionFunc<T> {
        let since_the_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        SubscriptionFunc {
            id: format!("{:?}{:?}", thread::current().id(), since_the_epoch),
            receiver: RawReceiver::new(func),

            #[cfg(feature = "for_futures")]
            streams: None,
            #[cfg(feature = "for_futures")]
            alive: None,
        }
    }
}

impl<T> UniqueId<String> for SubscriptionFunc<T> {
    fn get_id(&self) -> String {
        self.id.clone()
    }
}

impl<T: Send + Sync + 'static> PartialEq for SubscriptionFunc<T> {
    fn eq(&self, other: &SubscriptionFunc<T>) -> bool {
        self.id == other.id
    }
}

impl<T: Send + Sync + 'static> Subscription<T> for SubscriptionFunc<T> {
    fn on_next(&mut self, x: Arc<T>) {
        self.receiver.invoke(x.clone());

        #[cfg(feature = "for_futures")]
        {
            match &self.streams {
                Some(streams) => {
                    for item in streams.lock().as_mut().ok().unwrap().iter_mut() {
                        item.offer(Some(x.clone()));
                    }
                }
                None => {}
            }
        }
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
    /**
    Generate a new `RawReceiver` with the given `FnMut`.

    # Arguments

    * `func` - The given `FnMut`.

    */
    pub fn new(func: impl FnMut(Arc<T>) + Send + Sync + 'static) -> RawReceiver<T> {
        RawReceiver {
            func: Arc::new(Mutex::new(func)),
            _t: PhantomData,
        }
    }

    /**
    Invoke the `FnMut` with the given sent value.

    # Arguments

    * `x` - The sent value.

    */
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
    func: Arc<Mutex<dyn FnMut() + Send + Sync + 'static>>,
}

impl RawFunc {
    /**
    Generate a new `RawFunc` with the given `FnMut`.

    # Arguments

    * `func` - The given `FnMut`.

    */
    pub fn new<T>(func: T) -> RawFunc
    where
        T: FnMut() + Send + Sync + 'static,
    {
        RawFunc {
            func: Arc::new(Mutex::new(func)),
        }
    }

    /**
    Invoke the `FnMut`.
    */
    pub fn invoke(&self) {
        let func = &mut *self.func.lock().unwrap();
        (func)();
    }
}

#[test]
fn test_map_insert() {
    use std::collections::HashMap;

    let expected_by_ident = &mut HashMap::new();
    expected_by_ident.insert("a", "2");
    expected_by_ident.insert("b", "4");
    expected_by_ident.insert("c", "6");
    let actual = &mut HashMap::new();
    map_insert!(actual, [
        a : "2",
        b : "4",
        c : "6",
    ]);
    assert_eq!(expected_by_ident, actual);
    let actual = &mut HashMap::new();
    map_insert!(actual, {
        a : "2",
        b : "4",
        c : "6",
    });
    assert_eq!(expected_by_ident, actual);

    let expected = &mut HashMap::new();
    expected.insert("1", "2");
    expected.insert("3", "4");
    expected.insert("5", "6");

    let actual = &mut HashMap::new();
    map_insert!(actual, [
        "1" => "2",
        "3" => "4",
        "5" => "6",
    ]);
    assert_eq!(expected, actual);
    let actual = &mut HashMap::new();
    map_insert!(actual, {
        "1" => "2",
        "3" => "4",
        "5" => "6",
    });
    assert_eq!(expected, actual);

    let actual = &mut HashMap::new();
    map_insert!(actual, ["1", "2", "3", "4", "5", "6",]);
    assert_eq!(expected, actual);
}
