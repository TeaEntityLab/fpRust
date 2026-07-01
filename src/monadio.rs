//! Rx- / monad-inspired `MonadIO`: lazy effects, `map` / `fmap`, and subscription.
//!
//! Available with Cargo feature `monadio` (enabled by `pure`; depends on `sync` and `handler`).
//!
//! # Async (`for_futures`)
//!
//! With `for_futures` (as in `test_runtime`), `to_future` runs `eval` on the shared
//! thread pool. `observe_on` / `subscribe_on` route work through `Handler` threads;
//! README snippets that use `thread::sleep` after `start` are **demo timing only** —
//! prefer `CountDownLatch`, callbacks, or `to_future` for synchronization in production code.
use std::sync::{Arc, Mutex};

#[cfg(feature = "for_futures")]
use super::common::shared_thread_pool;
#[cfg(feature = "for_futures")]
use crate::futures::task::SpawnExt;
#[cfg(feature = "for_futures")]
use std::error::Error;

use super::handler::Handler;

use super::sync::CountDownLatch;

use super::common::{RawFunc, Subscription, SubscriptionFunc};

/// Lazy effect container with optional observe/subscribe handler routing.
///
/// # Type parameter
///
/// * `Y` — value produced when the effect runs
#[derive(Clone)]
pub struct MonadIO<Y> {
    effect: Arc<Mutex<dyn FnMut() -> Y + Send + Sync + 'static>>,
    ob_handler: Option<Arc<Mutex<dyn Handler>>>,
    sub_handler: Option<Arc<Mutex<dyn Handler>>>,
}

/// Wrap a value in a `FnMut` closure (used by `MonadIO::just`).
pub fn of<Z: 'static + Send + Sync + Clone>(r: Z) -> impl FnMut() -> Z + Send + Sync + 'static {
    let _r = Box::new(r);

    move || *_r.clone()
}

impl<Y: 'static + Send + Sync + Clone> From<Y> for MonadIO<Y> {
    fn from(r: Y) -> Self {
        MonadIO::just(r)
    }
}

impl<Y: 'static + Send + Sync + Clone> MonadIO<Y> {
    /// Lift a pure value into `MonadIO`.
    pub fn just(r: Y) -> MonadIO<Y> {
        MonadIO::new(of(r))
    }

    #[cfg(feature = "for_futures")]
    /// Run `eval` on the shared futures thread pool (`for_futures` only).
    ///
    /// # Examples
    ///
    /// ```
    /// use fp_rust::monadio::MonadIO;
    /// use futures::executor::block_on;
    /// use std::sync::Arc;
    ///
    /// let doubled = block_on(MonadIO::just(3).map(|i| i * 2).to_future());
    /// assert_eq!(Arc::new(6), doubled.ok().unwrap());
    /// ```
    pub async fn to_future(&self) -> Result<Arc<Y>, Box<dyn Error>> {
        // let mio = self.map(|y| y);
        let mio = self.clone();
        let future = {
            shared_thread_pool()
                .inner
                .lock()
                .unwrap()
                .spawn_with_handle(async move { mio.eval() })?
        };
        let result = future.await;
        Ok(result)
    }
}

impl<Y: 'static + Send + Sync> MonadIO<Y> {
    /// Create a `MonadIO` from an effect closure.
    pub fn new(effect: impl FnMut() -> Y + Send + Sync + 'static) -> MonadIO<Y> {
        MonadIO::new_with_handlers(effect, None, None)
    }

    /// Create with optional observe and subscribe `Handler` targets.
    pub fn new_with_handlers(
        effect: impl FnMut() -> Y + Send + Sync + 'static,
        ob: Option<Arc<Mutex<dyn Handler + 'static>>>,
        sub: Option<Arc<Mutex<dyn Handler + 'static>>>,
    ) -> MonadIO<Y> {
        MonadIO {
            effect: Arc::new(Mutex::new(effect)),
            ob_handler: ob,
            sub_handler: sub,
        }
    }

    /// Run the observe/subscribe pipeline on `h` when set (`Handler::post`).
    pub fn observe_on(&mut self, h: Option<Arc<Mutex<dyn Handler + 'static>>>) {
        self.ob_handler = h;
    }

    /// Deliver `on_next` on `h` when set; otherwise notify on the caller thread.
    pub fn subscribe_on(&mut self, h: Option<Arc<Mutex<dyn Handler + 'static>>>) {
        self.sub_handler = h;
    }

    /// Functor map over the produced value.
    pub fn map<Z: 'static + Send + Sync + Clone>(
        &self,
        func: impl FnMut(Y) -> Z + Send + Sync + 'static,
    ) -> MonadIO<Z> {
        let _func = Arc::new(Mutex::new(func));
        let mut _effect = self.effect.clone();

        MonadIO::new_with_handlers(
            move || (_func.lock().unwrap())((_effect.lock().unwrap())()),
            self.ob_handler.clone(),
            self.sub_handler.clone(),
        )
    }
    /// Monadic bind flattened into a single `MonadIO`.
    pub fn fmap<Z: 'static + Send + Sync + Clone>(
        &self,
        func: impl FnMut(Y) -> MonadIO<Z> + Send + Sync + 'static,
    ) -> MonadIO<Z> {
        let mut _func = Arc::new(Mutex::new(func));

        self.map(move |y: Y| ((_func.lock().unwrap())(y).effect.lock().unwrap())())
    }
    /// Run the effect and notify `s` (respecting observe/subscribe handlers).
    pub fn subscribe(&self, s: Arc<impl Subscription<Y>>) {
        let mut _effect = self.effect.clone();

        match &self.ob_handler {
            Some(ob_handler) => {
                let mut sub_handler_thread = Arc::new(self.sub_handler.clone());
                ob_handler.lock().unwrap().post(RawFunc::new(move || {
                    match Arc::make_mut(&mut sub_handler_thread) {
                        Some(sub_handler) => {
                            let effect = _effect.clone();
                            let s = s.clone();
                            sub_handler.lock().unwrap().post(RawFunc::new(move || {
                                let result = { Arc::new(effect.lock().unwrap()()) };
                                s.on_next(result);
                            }));
                        }
                        None => {
                            s.on_next(Arc::new(_effect.lock().unwrap()()));
                        }
                    }
                }));
            }
            None => {
                s.on_next(Arc::new(_effect.lock().unwrap()()));
            }
        }
    }
    /// `subscribe` with a plain `FnMut` callback.
    pub fn subscribe_fn(&self, func: impl FnMut(Arc<Y>) + Send + Sync + 'static) {
        self.subscribe(Arc::new(SubscriptionFunc::new(func)))
    }

    /// Run the effect once and block until a subscriber captures the result.
    pub fn eval(&self) -> Arc<Y> {
        let latch = CountDownLatch::new(1);
        let latch_thread = latch.clone();

        let result = Arc::new(Mutex::new(None::<Arc<Y>>));
        let result_thread = result.clone();
        self.subscribe_fn(move |y| {
            result_thread.lock().unwrap().replace(y);

            latch_thread.countdown();
        });

        latch.wait();
        let result = result.lock().as_mut().unwrap().to_owned();
        result.unwrap()
    }
}

#[cfg(feature = "for_futures")]
#[futures_test::test]
async fn test_monadio_async() {
    assert_eq!(Arc::new(3), MonadIO::just(3).eval());
    assert_eq!(
        Arc::new(3),
        MonadIO::just(3).to_future().await.ok().unwrap()
    );
    assert_eq!(
        Arc::new(6),
        MonadIO::just(3)
            .map(|i| i * 2)
            .to_future()
            .await
            .ok()
            .unwrap()
    );
}

#[test]
fn test_monadio_new() {
    use super::common::SubscriptionFunc;
    use super::handler::HandlerThread;
    use std::sync::Arc;

    use super::sync::CountDownLatch;

    let monadio_simple = MonadIO::just(3);
    // let mut monadio_simple = MonadIO::just(3);
    {
        assert_eq!(3, (monadio_simple.effect.lock().unwrap())());
    }
    let monadio_simple_map = monadio_simple.map(|x| x * 3);

    monadio_simple_map.subscribe_fn(move |x| {
        println!("monadio_simple_map {:?}", x);
        assert_eq!(9, *Arc::make_mut(&mut x.clone()));
    });

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

    // Waiting for being unlcoked
    latch.clone().wait();
}

#[test]
fn test_monadio_of_returns_value() {
    let mut f = of(7);
    assert_eq!(7, f());
    // of yields a fresh clone each call.
    assert_eq!(7, f());
}

#[test]
fn test_monadio_just_eval() {
    // just + eval round-trips the value synchronously (no handlers).
    assert_eq!(Arc::new(3), MonadIO::just(3).eval());
    assert_eq!(
        Arc::new(String::from("x")),
        MonadIO::just(String::from("x")).eval()
    );
}

#[test]
fn test_monadio_from_trait() {
    let m: MonadIO<i32> = MonadIO::from(42);
    assert_eq!(Arc::new(42), m.eval());
}

#[test]
fn test_monadio_new_with_effect() {
    // new() takes a raw effect closure.
    let m = MonadIO::new(|| 5 + 5);
    assert_eq!(Arc::new(10), m.eval());
}

#[test]
fn test_monadio_map() {
    let m = MonadIO::just(3).map(|x| x * 2);
    assert_eq!(Arc::new(6), m.eval());
}

#[test]
fn test_monadio_map_changes_type() {
    let m = MonadIO::just(8).map(|x| x.to_string());
    assert_eq!(Arc::new(String::from("8")), m.eval());
}

#[test]
fn test_monadio_map_chained() {
    let m = MonadIO::just(2).map(|x| x + 1).map(|x| x * 10);
    assert_eq!(Arc::new(30), m.eval());
}

#[test]
fn test_monadio_fmap_flattens() {
    // fmap's closure returns another MonadIO which is flattened.
    let m = MonadIO::just(4).fmap(|x| MonadIO::new(move || x * 5));
    assert_eq!(Arc::new(20), m.eval());
}

#[test]
fn test_monadio_fmap_then_map() {
    let m = MonadIO::just(1)
        .fmap(|x| MonadIO::new(move || x * 4))
        .map(|x| x * 3)
        .map(|x| x * 3);
    // 1 -> 4 -> 12 -> 36
    assert_eq!(Arc::new(36), m.eval());
}

#[test]
fn test_monadio_subscribe_fn_sync() {
    use std::sync::atomic::{AtomicI32, Ordering};

    // Without handlers, subscribe_fn runs synchronously on the current thread.
    let captured = Arc::new(AtomicI32::new(0));
    let captured_thread = captured.clone();
    MonadIO::just(11).subscribe_fn(move |x| {
        captured_thread.store(*x, Ordering::SeqCst);
    });
    assert_eq!(11, captured.load(Ordering::SeqCst));
}

#[test]
fn test_monadio_subscribe_sync() {
    use super::common::SubscriptionFunc;
    use std::sync::atomic::{AtomicI32, Ordering};

    let captured = Arc::new(AtomicI32::new(0));
    let captured_thread = captured.clone();
    let sub = Arc::new(SubscriptionFunc::new(move |x: Arc<i32>| {
        captured_thread.store(*x, Ordering::SeqCst);
    }));
    MonadIO::just(5).map(|x| x * 4).subscribe(sub);
    assert_eq!(20, captured.load(Ordering::SeqCst));
}

#[test]
fn test_monadio_eval_runs_effect_each_time() {
    use std::sync::atomic::{AtomicI32, Ordering};

    // Each eval re-runs the underlying effect.
    let counter = Arc::new(AtomicI32::new(0));
    let counter_thread = counter.clone();
    let m = MonadIO::new(move || counter_thread.fetch_add(1, Ordering::SeqCst) + 1);
    assert_eq!(Arc::new(1), m.eval());
    assert_eq!(Arc::new(2), m.eval());
    assert_eq!(2, counter.load(Ordering::SeqCst));
}
