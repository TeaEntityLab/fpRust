//! Shared traits, structs, and helpers used across `fp_rust` modules.
//!
//! Includes observer/pub-sub primitives, thread-safe queues, ID generation,
//! and small utilities such as [`get_mut`] and the `map_insert!` macro.

use std::cmp::PartialEq;
use std::collections::LinkedList;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};
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
use std::sync::{atomic::AtomicBool, Once};
#[cfg(feature = "for_futures")]
use std::task::{Context, Poll, Waker};

// pub trait FnMutReceiveThreadSafe<X>: FnMut(Arc<X>) + Send + Sync + 'static {}
// pub trait FnMutReturnThreadSafe<X>: FnMut() -> X + Send + Sync + 'static {}

/// Handle to the process-wide shared [`ThreadPool`].
///
/// Clones share the same underlying pool. Obtain one via [`shared_thread_pool`].
#[cfg(feature = "for_futures")]
#[derive(Clone)]
pub struct SharedThreadPoolReader {
    // Since we will be used in many threads, we need to protect
    // concurrent access
    /// Shared executor protected by a mutex for concurrent access.
    pub inner: Arc<Mutex<ThreadPool>>,
}

/// Returns a clone of the lazily initialized process-wide thread pool.
#[cfg(feature = "for_futures")]
pub fn shared_thread_pool() -> SharedThreadPoolReader {
    // Initialize it to a null value
    static mut SINGLETON: *const SharedThreadPoolReader =
        std::ptr::null::<SharedThreadPoolReader>();
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
            SINGLETON = mem::transmute::<Box<SharedThreadPoolReader>, *const SharedThreadPoolReader>(Box::new(singleton));
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

/// Returns a mutable reference to the element at `index` in `v`, if in range.
///
/// # Examples
///
/// ```
/// use fp_rust::common::get_mut;
///
/// let mut v = vec![10, 20, 30];
/// if let Some(elem) = get_mut(&mut v, 1) {
///     *elem = 99;
/// }
/// assert_eq!(vec![10, 99, 30], v);
/// assert_eq!(None, get_mut(&mut v, 5));
/// ```
pub fn get_mut<T>(v: &mut Vec<T>, index: usize) -> Option<&mut T> {
    for (i, elem) in v.iter_mut().enumerate() {
        if index == i {
            return Some(elem);
        }
    }

    None
}

/// Thread-safe FIFO queue backed by a [`LinkedList`].
///
/// Clones share the same underlying list (`Arc<Mutex<..>>`). With the
/// `for_futures` feature, the type also implements [`Stream`].
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
    /// Creates an empty queue.
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

    /// Appends `input` to the back of the queue.
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
        }

        #[cfg(not(feature = "for_futures"))]
        {
            self.inner.lock().unwrap().push_back(input);
        }
    }

    /// Removes and returns the front element, or `None` when empty.
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

    /// Marks the stream finished; [`Stream::poll_next`] will eventually return `None`.
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

        // {
        //     picked = self.pop_front();
        // }
        let picked: Option<T> = self.pop_front();
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
        Poll::Ready(None)
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

*NOTE*: Inspired by and modified from
<https://github.com/eliovir/rust-examples/blob/master/design_pattern-observer.rs>.
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

static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Generates a process-unique string identifier.
///
/// Combines thread id, wall-clock time, and a monotonic counter so that
/// back-to-back calls on the same thread still produce distinct values.
pub fn generate_id() -> String {
    let since_the_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    // A process-wide monotonic counter guarantees uniqueness even for
    // same-thread, same-tick, back-to-back calls: SystemTime resolution is
    // coarse, so thread id + timestamp alone collide (see UniqueId).
    let seq = ID_COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("{:?}{:?}-{}", thread::current().id(), since_the_epoch, seq)
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
    /// Callback invoked when [`Subscription::on_next`] receives a value.
    pub receiver: RawReceiver<T>,

    #[cfg(feature = "for_futures")]
    cached: LinkedListAsync<Arc<T>>,
}

impl<T> SubscriptionFunc<T> {
    fn generate_id() -> String {
        generate_id()
    }

    /// Creates a subscription that forwards values to `func`.
    pub fn new(func: impl FnMut(Arc<T>) + Send + Sync + 'static) -> SubscriptionFunc<T> {
        // The internal `cached` stream starts CLOSED: buffering begins only when
        // `as_stream()` opens it. A callback-only subscription (the common case)
        // has no stream consumer, so buffering from construction would leak
        // every delivered value forever.
        #[cfg(feature = "for_futures")]
        let cached = {
            let mut cached = LinkedListAsync::new();
            cached.close_stream();
            cached
        };

        SubscriptionFunc {
            id: Self::generate_id(),
            receiver: RawReceiver::new(func),

            #[cfg(feature = "for_futures")]
            cached,
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
    /// Stops buffering further values on the internal async stream.
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

    /// Returns a clone of the internal queue as an open [`Stream`].
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

#[test]
fn test_common_get_mut() {
    let mut v = vec![10, 20, 30];

    // Valid index hands back a mutable reference we can write through.
    if let Some(elem) = get_mut(&mut v, 1) {
        *elem = 99;
    }
    assert_eq!(vec![10, 99, 30], v);

    // Out-of-range index yields None.
    assert_eq!(None, get_mut(&mut v, 5));

    // Empty vec yields None at index 0.
    let mut empty = Vec::<i32>::new();
    assert_eq!(None, get_mut(&mut empty, 0));
}

#[test]
fn test_common_generate_id_non_empty_and_changes() {
    use std::collections::HashSet;

    let a = generate_id();
    assert_eq!(false, a.is_empty());

    // UniqueId contract: ids must be unique even for tight, same-thread,
    // same-tick, back-to-back calls (SystemTime resolution alone is too
    // coarse to guarantee this; a monotonic counter does).
    let n = 10_000;
    let mut seen = HashSet::new();
    for _ in 0..n {
        assert_eq!(true, seen.insert(generate_id()));
    }
    assert_eq!(n, seen.len());
}

#[test]
fn test_common_linked_list_async_fifo() {
    let list = LinkedListAsync::<i32>::new();
    list.push_back(1);
    list.push_back(2);
    list.push_back(3);

    assert_eq!(Some(1), list.pop_front());
    assert_eq!(Some(2), list.pop_front());
    assert_eq!(Some(3), list.pop_front());
    // Draining past the end returns None.
    assert_eq!(None, list.pop_front());
}

#[cfg(feature = "for_futures")]
#[test]
fn test_common_linked_list_async_size_hint_never_lies() {
    use futures::stream::Stream;

    // size_hint's upper bound is a promise. A live stream can still receive
    // items via push_back, so it must NOT report Some(0) ("finished") — that
    // lets Stream combinators drop pending items. A closed stream must not be
    // reported as falsely unbounded either. (0, None) is correct for both.
    let mut list = LinkedListAsync::<i32>::new();
    assert_eq!((0, None), Stream::size_hint(&list));

    list.close_stream();
    assert_eq!((0, None), Stream::size_hint(&list));
}

#[test]
fn test_common_linked_list_async_clone_shares_state() {
    // Clones share the same underlying list (Arc<Mutex<..>> inside).
    let list = LinkedListAsync::<i32>::new();
    let clone = list.clone();

    list.push_back(42);
    assert_eq!(Some(42), clone.pop_front());

    clone.push_back(7);
    assert_eq!(Some(7), list.pop_front());
}

#[test]
fn test_common_linked_list_async_default() {
    use std::sync::Arc;

    let list: LinkedListAsync<Arc<i32>> = Default::default();
    list.push_back(Arc::new(5));
    assert_eq!(Some(Arc::new(5)), list.pop_front());
}

#[test]
fn test_common_raw_func_invokes_closure() {
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    let counter = Arc::new(AtomicI32::new(0));
    let counter_thread = counter.clone();
    let f = RawFunc::new(move || {
        counter_thread.fetch_add(5, Ordering::SeqCst);
    });

    f.invoke();
    assert_eq!(5, counter.load(Ordering::SeqCst));
    // RawFunc holds FnMut; invoking again accumulates.
    f.invoke();
    assert_eq!(10, counter.load(Ordering::SeqCst));
}

#[test]
fn test_common_raw_receiver_passes_value() {
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    let seen = Arc::new(AtomicI32::new(0));
    let seen_thread = seen.clone();
    let receiver = RawReceiver::new(move |x: Arc<i32>| {
        seen_thread.store(*x, Ordering::SeqCst);
    });

    receiver.invoke(Arc::new(123));
    assert_eq!(123, seen.load(Ordering::SeqCst));
}

#[test]
fn test_common_subscription_func_on_next() {
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;

    let total = Arc::new(AtomicI32::new(0));
    let total_thread = total.clone();
    let sub = SubscriptionFunc::new(move |x: Arc<i32>| {
        total_thread.fetch_add(*x, Ordering::SeqCst);
    });

    sub.on_next(Arc::new(3));
    sub.on_next(Arc::new(4));
    assert_eq!(7, total.load(Ordering::SeqCst));
}

#[test]
fn test_common_subscription_func_clone_equal_same_id() {
    let sub = SubscriptionFunc::new(|_x: Arc<i32>| {});
    let cloned = sub.clone();

    // Clone preserves identity (id is copied, not regenerated).
    assert_eq!(sub.get_id(), cloned.get_id());
    assert_eq!(true, sub == cloned);
    assert_eq!(false, sub.get_id().is_empty());
}

#[test]
fn test_common_subscription_func_distinct_ids() {
    // Back-to-back construction (no sleep): independently-constructed
    // subscriptions must have distinct ids, which is what makes PartialEq
    // and Publisher::delete_observer correct. (The reliable regression
    // guard for the underlying same-tick collision is the tight-loop
    // test on generate_id itself.)
    let a = SubscriptionFunc::new(|_x: Arc<i32>| {});
    let b = SubscriptionFunc::new(|_x: Arc<i32>| {});

    assert_eq!(true, a.get_id() != b.get_id());
    assert_eq!(false, a == b);
}

#[cfg(feature = "for_futures")]
#[test]
fn test_common_subscription_func_no_buffer_until_stream_opened() {
    // Regression: a callback-only SubscriptionFunc must NOT buffer values into
    // its internal cached stream. Nothing consumes that buffer until as_stream()
    // is called, so buffering from construction is an unbounded memory leak.
    let sub = SubscriptionFunc::new(|_x: Arc<i32>| {});

    // Deliver values as a plain callback subscriber (the common case).
    sub.on_next(Arc::new(1));
    sub.on_next(Arc::new(2));
    sub.on_next(Arc::new(3));

    // The cached stream stays empty: no stream was ever opened.
    assert_eq!(None, sub.cached.pop_front());

    // Opening the stream begins buffering from that point on.
    let mut sub = sub;
    let stream = sub.as_stream();
    sub.on_next(Arc::new(4));
    sub.on_next(Arc::new(5));
    assert_eq!(Some(Arc::new(4)), stream.pop_front());
    assert_eq!(Some(Arc::new(5)), stream.pop_front());
}
