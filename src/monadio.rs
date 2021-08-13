/*!
In this module there're implementations & tests of `MonadIO`.
It's inspired by `Rx` & `MonadIO` in `Haskell`
*/
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

/**
`MonadIO` implements basic `Rx`/`MonadIO` APIs.
The `observe` and `subscribe` actions could be sync/async,
and `observe` & `subscribe` could be on other `thread`s
(by setting up `observe_on` and `subscribe_on`).

# Arguments

* `Y` - The generic type of data

# Remarks

It's inspired by `Rx` & `MonadIO` in `Haskell`
, and easily run it on sync/async scenaios.

``
*/
#[derive(Clone)]
pub struct MonadIO<Y> {
    effect: Arc<Mutex<dyn FnMut() -> Y + Send + Sync + 'static>>,
    ob_handler: Option<Arc<Mutex<dyn Handler>>>,
    sub_handler: Option<Arc<Mutex<dyn Handler>>>,
}

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
    pub fn just(r: Y) -> MonadIO<Y> {
        MonadIO::new(of(r))
    }

    #[cfg(feature = "for_futures")]
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
    pub fn new(effect: impl FnMut() -> Y + Send + Sync + 'static) -> MonadIO<Y> {
        MonadIO::new_with_handlers(effect, None, None)
    }

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

    pub fn observe_on(&mut self, h: Option<Arc<Mutex<dyn Handler + 'static>>>) {
        self.ob_handler = h;
    }

    pub fn subscribe_on(&mut self, h: Option<Arc<Mutex<dyn Handler + 'static>>>) {
        self.sub_handler = h;
    }

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
    pub fn fmap<Z: 'static + Send + Sync + Clone>(
        &self,
        func: impl FnMut(Y) -> MonadIO<Z> + Send + Sync + 'static,
    ) -> MonadIO<Z> {
        let mut _func = Arc::new(Mutex::new(func));

        self.map(move |y: Y| ((_func.lock().unwrap())(y).effect.lock().unwrap())())
    }
    pub fn subscribe(&self, s: Arc<impl Subscription<Y>>) {
        let mut _effect = self.effect.clone();

        match &self.ob_handler {
            Some(ob_handler) => {
                let mut sub_handler_thread = Arc::new(self.sub_handler.clone());
                ob_handler.lock().unwrap().post(RawFunc::new(move || {
                    match Arc::make_mut(&mut sub_handler_thread) {
                        Some(ref mut sub_handler) => {
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
    pub fn subscribe_fn(&self, func: impl FnMut(Arc<Y>) + Send + Sync + 'static) {
        self.subscribe(Arc::new(SubscriptionFunc::new(func)))
    }

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
    use std::{thread, time};

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
}
