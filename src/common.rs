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
use std::mem;
#[cfg(feature = "for_futures")]
use std::pin::Pin;
#[cfg(feature = "for_futures")]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Once,
};
#[cfg(feature = "for_futures")]
use std::task::{Context, Poll, Waker};

#[cfg(feature = "for_futures")]
use futures::executor::ThreadPool;
#[cfg(feature = "for_futures")]
use futures::stream::Stream;

// pub trait FnMutReceiveThreadSafe<X>: FnMut(Arc<X>) + Send + Sync + 'static {}
// pub trait FnMutReturnThreadSafe<X>: FnMut() -> X + Send + Sync + 'static {}

#[cfg(feature = "for_futures")]
#[derive(Clone)]
pub struct SharedThreadPoolReader {
    // Since we will be used in many threads, we need to protect
    // concurrent access
    pub inner: Arc<Mutex<ThreadPool>>,
}
#[cfg(feature = "for_futures")]
pub fn shared_thread_pool() -> SharedThreadPoolReader {
    // Initialize it to a null value
    static mut SINGLETON: *const SharedThreadPoolReader = 0 as *const SharedThreadPoolReader;
    static ONCE: Once = Once::new();

    unsafe {
        ONCE.call_once(|| {
            // Make it
            let singleton = SharedThreadPoolReader {
                inner: Arc::new(Mutex::new(
                    ThreadPool::new().expect("Unable to create threadpool"),
                )),
            };

            // Put it in the heap so it can outlive this call
            SINGLETON = mem::transmute(Box::new(singleton));
        });

        // Now we give out a copy of the data that is safe to use concurrently.
        (*SINGLETON).clone()
    }
}

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
// #[derive(Clone)]
pub struct SubscriptionFunc<T> {
    id: String,
    pub receiver: RawReceiver<T>,

    #[cfg(feature = "for_futures")]
    cached: Option<Arc<Mutex<VecDeque<Arc<T>>>>>,
    #[cfg(feature = "for_futures")]
    alive: Option<Arc<Mutex<AtomicBool>>>,
    #[cfg(feature = "for_futures")]
    waker: Arc<Mutex<Option<Waker>>>,
}

impl<T> SubscriptionFunc<T> {
    fn generate_id() -> String {
        let since_the_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        format!("{:?}{:?}", thread::current().id(), since_the_epoch)
    }
}

impl<T> Clone for SubscriptionFunc<T> {
    fn clone(&self) -> Self {
        SubscriptionFunc {
            id: Self::generate_id(),
            receiver: RawReceiver {
                func: self.receiver.func.clone(),
                _t: PhantomData,
            },
            #[cfg(feature = "for_futures")]
            cached: self.cached.clone(),
            #[cfg(feature = "for_futures")]
            alive: self.alive.clone(),
            #[cfg(feature = "for_futures")]
            waker: self.waker.clone(),
        }
    }
}

#[cfg(feature = "for_futures")]
impl<T> SubscriptionFunc<T> {
    pub fn close_stream(&mut self) {
        if let Some(alive) = &self.alive {
            {
                alive.lock().unwrap().store(false, Ordering::SeqCst);
            }
            self.alive = None;
        }

        // let old_cached = self.cached.clone();
        if self.cached.is_some() {
            self.cached = None;
        }
        /*
        if let Some(cached) = &old_cached {
            {
                cached.lock().unwrap().clear();
            }
        }
        // */

        {
            if let Some(waker) = self.waker.clone().lock().unwrap().take() {
                self.waker = Arc::new(Mutex::new(None));
                waker.wake();
            }
        }
    }
}

#[cfg(feature = "for_futures")]
impl<T> SubscriptionFunc<T>
where
    T: 'static + Send + Unpin,
{
    fn open_stream(&mut self) {
        match &self.alive {
            Some(alive) => {
                alive.lock().unwrap().store(true, Ordering::SeqCst);
            }
            None => {
                self.alive = Some(Arc::new(Mutex::new(AtomicBool::new(true))));
            }
        }

        if self.cached.is_none() {
            self.cached = Some(Arc::new(Mutex::new(VecDeque::new())));
        }
    }

    pub fn as_stream(&mut self) -> SubscriptionFuncStream<T> {
        self.open_stream();

        SubscriptionFuncStream { 0: self.clone() }
    }
}

impl<T: Send + Sync + 'static> SubscriptionFunc<T> {
    pub fn new(func: impl FnMut(Arc<T>) + Send + Sync + 'static) -> SubscriptionFunc<T> {
        SubscriptionFunc {
            id: Self::generate_id(),
            receiver: RawReceiver::new(func),

            #[cfg(feature = "for_futures")]
            cached: None,
            #[cfg(feature = "for_futures")]
            alive: None,
            #[cfg(feature = "for_futures")]
            waker: Arc::new(Mutex::new(None)),
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
            if let Some(alive) = &self.alive {
                if let Some(cached) = &self.cached {
                    let alive = { alive.lock().unwrap().load(Ordering::SeqCst) };
                    if alive {
                        {
                            cached.lock().unwrap().push_back(x.clone())
                        };
                        {
                            if let Some(waker) = self.waker.lock().unwrap().take() {
                                waker.wake()
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(feature = "for_futures")]
#[derive(Clone)]
pub struct SubscriptionFuncStream<T>(SubscriptionFunc<T>);

#[cfg(feature = "for_futures")]
impl<T> SubscriptionFuncStream<T> {
    pub fn close_stream(&mut self) {
        self.0.close_stream();
    }
}

/*
#[cfg(feature = "for_futures")]
impl<T> SubscriptionFuncStream<T>
where
    T: 'static + Send + Unpin,
{
    pub fn open_stream(&mut self) {
        self.0.open_stream();
    }
}
*/

#[cfg(feature = "for_futures")]
impl<T> Stream for SubscriptionFuncStream<T>
where
    T: 'static + Send + Unpin,
{
    type Item = Arc<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.0.alive.is_none() && self.0.cached.is_none() {
            return Poll::Ready(None);
        }

        if let Some(cached) = &self.0.cached {
            let picked: Option<Arc<T>>;
            {
                picked = cached.lock().unwrap().pop_front();
            }
            if picked.is_some() {
                return Poll::Ready(picked);
            }
        }

        // Check alive
        if let Some(alive) = &self.0.alive {
            // Check alive
            let alive = { alive.lock().unwrap().load(Ordering::SeqCst) };
            if alive {
                // Check cached
                if let Some(cached) = &self.0.cached {
                    let picked: Option<Arc<T>>;
                    {
                        picked = cached.lock().unwrap().pop_front();
                    }

                    // Check Pending(None) or Ready(Some(item))
                    if picked.is_none() {
                        // Keep Pending
                        {
                            self.0.waker.lock().unwrap().replace(cx.waker().clone());
                        };
                        return Poll::Pending;
                    }
                    return Poll::Ready(picked);
                }
                return Poll::Ready(None);
            }
            return Poll::Ready(None);
        }
        return Poll::Ready(None);
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.0.alive.is_some() && self.0.cached.is_some() {
            if let Some(alive) = &self.0.alive {
                // Check alive
                let alive = { alive.lock().unwrap().load(Ordering::SeqCst) };
                if alive {
                    return (0, Some(0));
                }
                return (0, None);
            }
            return (0, None);
        } else {
            return (0, None);
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
