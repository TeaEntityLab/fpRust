/*!
In this module, there're many `trait`s/`struct`s and `fn`s defined,
for general purposes crossing over many modules of `fpRust`.
*/

use std::cmp::PartialEq;
use std::collections::LinkedList;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "for_futures")]
use futures::executor::ThreadPool;
#[cfg(feature = "for_futures")]
use futures::stream::Stream;
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

#[derive(Debug, Clone)]
pub struct LinkedListAsync<T> {
    inner: Arc<Mutex<LinkedList<T>>>,

    #[cfg(feature = "for_futures")]
    alive: Arc<Mutex<AtomicBool>>,
    #[cfg(feature = "for_futures")]
    waker: Arc<Mutex<Option<Waker>>>,

    _t: PhantomData<T>,
}

impl<T> LinkedListAsync<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(LinkedList::new())),

            #[cfg(feature = "for_futures")]
            alive: Arc::new(Mutex::new(AtomicBool::new(true))),
            #[cfg(feature = "for_futures")]
            waker: Arc::new(Mutex::new(None)),

            _t: PhantomData,
        }
    }

    pub fn push_back(&self, input: T) {
        #[cfg(feature = "for_futures")]
        {
            {
                let alive = self.alive.lock().unwrap();
                if alive.load(Ordering::SeqCst) {
                    self.inner.lock().unwrap().push_back(input);
                }

                self.wake();
            }

            return;
        }

        #[cfg(not(feature = "for_futures"))]
        {
            self.inner.lock().unwrap().push_back(input);
        }
    }

    pub fn pop_front(&self) -> Option<T> {
        self.inner.lock().unwrap().pop_front()
    }

    #[cfg(feature = "for_futures")]
    #[inline]
    fn wake(&self) {
        let mut waker = self.waker.lock().unwrap();
        if let Some(waker) = waker.take() {
            waker.wake();
        }
    }

    #[cfg(feature = "for_futures")]
    fn open_stream(&mut self) {
        self.alive.lock().unwrap().store(true, Ordering::SeqCst);
    }

    #[cfg(feature = "for_futures")]
    pub fn close_stream(&mut self) {
        {
            let alive = self.alive.lock().unwrap();
            alive.store(false, Ordering::SeqCst);

            self.wake()
        }
    }
}

#[cfg(feature = "for_futures")]
impl<T> Stream for LinkedListAsync<T>
where
    T: 'static + Send + Unpin,
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // let mut inner = self.inner.lock().unwrap();
        let alive = self.alive.lock().unwrap();
        let mut waker = self.waker.lock().unwrap();

        let picked: Option<T>;
        // {
        //     picked = self.pop_front();
        // }
        picked = self.pop_front();
        if picked.is_some() {
            return Poll::Ready(picked);
        }

        // Check alive
        if alive.load(Ordering::SeqCst) {
            // Check cached
            // let picked: Option<T>;
            // picked = inner.pop_front();

            // Check Pending(None) or Ready(Some(item))
            // if picked.is_none() {
            // Keep Pending
            {
                waker.replace(cx.waker().clone());
            };
            return Poll::Pending;
            // }
            // return Poll::Ready(picked);
        }
        return Poll::Ready(None);
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Check alive
        let alive = self.alive.lock().unwrap();
        if alive.load(Ordering::SeqCst) {
            return (0, Some(0));
        }
        return (0, None);
    }
}

impl<T> Default for LinkedListAsync<Arc<T>> {
    fn default() -> Self {
        Self::new()
    }
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
    fn add_observer(&mut self, observer: Arc<T>);

    /**
    Remove the observer.

    # Arguments

    * `observer` - The given `Subscription`.

    */
    fn delete_observer(&mut self, observer: Arc<T>);

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
    fn on_next(&self, x: Arc<X>);
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

pub fn generate_id() -> String {
    let since_the_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    format!("{:?}{:?}", thread::current().id(), since_the_epoch)
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
    cached: LinkedListAsync<Arc<T>>,
}

impl<T> SubscriptionFunc<T> {
    fn generate_id() -> String {
        generate_id()
    }

    pub fn new(func: impl FnMut(Arc<T>) + Send + Sync + 'static) -> SubscriptionFunc<T> {
        SubscriptionFunc {
            id: Self::generate_id(),
            receiver: RawReceiver::new(func),

            #[cfg(feature = "for_futures")]
            cached: LinkedListAsync::new(),
        }
    }
}

impl<T> Clone for SubscriptionFunc<T> {
    fn clone(&self) -> Self {
        SubscriptionFunc {
            // id: Self::generate_id(),
            id: self.id.clone(),
            receiver: RawReceiver {
                func: self.receiver.func.clone(),
                _t: PhantomData,
            },
            #[cfg(feature = "for_futures")]
            cached: self.cached.clone(),
        }
    }
}

#[cfg(feature = "for_futures")]
impl<T> SubscriptionFunc<T> {
    pub fn close_stream(&mut self) {
        self.cached.close_stream();
    }
}

#[cfg(feature = "for_futures")]
impl<T> SubscriptionFunc<T>
where
    T: Unpin,
{
    fn open_stream(&mut self) {
        self.cached.open_stream();
    }

    pub fn as_stream(&mut self) -> LinkedListAsync<Arc<T>> {
        self.open_stream();

        self.cached.clone()
    }
}

impl<T> UniqueId<String> for SubscriptionFunc<T> {
    fn get_id(&self) -> String {
        self.id.clone()
    }
}

impl<T> PartialEq for SubscriptionFunc<T> {
    fn eq(&self, other: &SubscriptionFunc<T>) -> bool {
        self.id == other.id
    }
}

impl<T: Send + Sync + 'static> Subscription<T> for SubscriptionFunc<T> {
    fn on_next(&self, x: Arc<T>) {
        self.receiver.invoke(x.clone());

        #[cfg(feature = "for_futures")]
        {
            self.cached.push_back(x);
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
        (self.func.lock().unwrap())(x);
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
        // let func = &mut *self.func.lock().unwrap();
        (self.func.lock().unwrap())();
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
